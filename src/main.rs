use std::{
    collections::{HashMap, hash_map::Entry},
    error::Error,
    fmt,
    fs::File,
    io::Read,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
    str::FromStr,
};

use headers_accept::Accept;
use http::uri::Authority;
use http_wasm_guest::{Guest, Request, Response, api::Bytes, host::config, register};
use mediatype::MediaType;
use serde::{
    Deserialize, Deserializer,
    de::{self, SeqAccess, Visitor},
};
use wirefilter::{ExecutionContext, Scheme};

#[macro_use]
extern crate log;

struct Filter {
    scheme: Scheme,
    filter: wirefilter::Filter,
}

const MAINTENACE_STATUS: i32 = 503;

impl Filter {
    fn new(filter: &str) -> Result<Self, Box<dyn Error>> {
        let scheme_builder = Scheme! {
            http.method: Bytes,
            http.ua: Bytes,
            http.host: Bytes,
            http.path: Bytes,
            http.version: Bytes,
            src.ip: Ip,
            src.port: Int,
        };
        let scheme = scheme_builder.build();
        let ast = scheme.parse(filter).map_err(|e| e.to_string())?;
        let filter = ast.compile();
        Ok(Self { scheme, filter })
    }

    fn matches(&self, request: &Request) -> Result<bool, Box<dyn Error>> {
        let mut context = ExecutionContext::new(&self.scheme);
        let method = request.method();
        let method = method.to_str().unwrap_or_default();
        debug!("Setting method: {method}");
        context
            .set_field_value_from_name("http.method", method)
            .expect("Failed to set method");

        let header = request.header();

        let user_agents = header.values(&Bytes::from("User-Agent"));
        if let Some(agent) = user_agents.first() {
            let agent = agent.to_str().unwrap_or_default();
            debug!("Setting User-Agent: {agent}");
            context
                .set_field_value_from_name("http.ua", agent)
                .expect("Failed to set User-Agent");
            if user_agents.len() > 1 {
                debug!("Multiple User-Agent headers found, using the first one: {agent}");
            }
        } else {
            context
                .set_field_value_from_name("http.ua", "")
                .expect("Failed to set User-Agent");
        }

        let hosts = header.values(&Bytes::from("Host"));
        if let Some(authority) = hosts
            .first()
            .and_then(|h| Authority::from_str(h.to_str().unwrap_or_default()).ok())
        {
            let host = authority.host().to_string();
            if hosts.len() > 1 {
                debug!(
                    "Multiple Host headers found, using the first one: {}",
                    &host
                );
            }
            debug!("Setting Host: {}", &host);
            context
                .set_field_value_from_name("http.host", host)
                .expect("Failed to set Host");
        } else {
            context
                .set_field_value_from_name("http.host", "")
                .expect("Failed to set Host");
        }

        let version = request.version();
        let version = version.to_str().unwrap_or_default();
        debug!("Setting HTTP version: {version}");
        context
            .set_field_value_from_name("http.version", version)
            .expect("Failed to set HTTP version");

        let source_addr = request.source_addr();
        let source_addr = source_addr.to_str().unwrap_or_default();
        if let Ok(addr) = SocketAddr::from_str(source_addr) {
            debug!("Setting source address: {addr:?}");
            context
                .set_field_value_from_name("src.port", addr.port())
                .expect("Failed to set source port");
            context
                .set_field_value_from_name("src.ip", addr.ip())
                .expect("Failed to set source port");
        } else if let Ok(ip) = IpAddr::from_str(source_addr) {
            debug!("Setting client IP: {ip}");
            context
                .set_field_value_from_name("src.ip", ip)
                .expect("Failed to set client IP");
        } else {
            debug!("Invalid source address: {source_addr}");
            let unspecified_ip = IpAddr::V4(Ipv4Addr::UNSPECIFIED);
            context
                .set_field_value_from_name("src.ip", unspecified_ip)
                .expect("Failed to set client IP");
        }

        let uri = request.uri();
        let path = uri.to_str().unwrap_or_default();
        debug!("Setting path: {path}");
        context
            .set_field_value_from_name("http.path", path)
            .expect("Failed to set path");

        self.filter.execute(&context).map_err(Into::into)
    }
}

struct MaintenancePages {
    pages: HashMap<MediaType<'static>, PathBuf>,
}

impl MaintenancePages {
    fn new(paths: Vec<PathBuf>) -> Self {
        let mut pages = HashMap::with_capacity(paths.len());
        for page in paths {
            match file_type::FileType::try_from_file(&page) {
                Ok(mime) => {
                    for media_type_name in mime.media_types() {
                        let mime_type = match MediaType::parse(media_type_name) {
                            Ok(mt) => mt,
                            Err(e) => {
                                error!("Failed to parse MIME type for {page:?}: {e}");
                                continue;
                            }
                        };

                        if page.is_file() {
                            let entry = pages.entry(mime_type.clone());
                            if matches!(&entry, Entry::Occupied(_)) {
                                error!(
                                    "Duplicate maintenance page for MIME type {mime_type}, skipping {page:?}"
                                );
                                continue;
                            }
                            info!("Adding maintenance page for MIME type {mime_type}: {page:?}");
                            entry.insert_entry(page.clone());
                        } else {
                            error!("Path {page:?} is not a file, skipping");
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to infer MIME type for {page:?}: {e}");
                    continue;
                }
            };
        }
        Self { pages }
    }

    fn send_page(&self, accept_header: &str, response: &Response) -> Result<(), Box<dyn Error>> {
        let accept = match Accept::from_str(accept_header) {
            Ok(a) => a,
            Err(e) => {
                debug!("Failed to parse Accept header: {e}");
                return fallback(response);
            }
        };
        let available = self.pages.keys();
        let Some(mime) = accept.negotiate(available) else {
            return fallback(response);
        };
        let Some(path) = self.pages.get(mime) else {
            error!("No maintenance page available for MIME type: {mime}");
            return fallback(response);
        };

        // Read and send the file content in chunks
        match File::open(path) {
            Ok(mut file) => {
                response.header().set(
                    &Bytes::from("Content-Type"),
                    &Bytes::from(mime.to_string().as_str()),
                );
                response.set_status(MAINTENACE_STATUS);
                let mut buffer = [0u8; 8192];
                let body = response.body();
                loop {
                    match file.read(&mut buffer)? {
                        0 => break, // EOF
                        n => {
                            body.write(&Bytes::from(&buffer[..n]));
                        }
                    }
                }
                Ok(())
            }
            Err(e) => {
                error!("Failed to open maintenance page {}: {}", path.display(), e);
                fallback(response)
            }
        }
    }
}

fn fallback(response: &Response) -> Result<(), Box<dyn Error>> {
    response
        .header()
        .set(&Bytes::from("Content-Type"), &Bytes::from("text/plain"));
    response.set_status(MAINTENACE_STATUS);
    response
        .body()
        .write(&Bytes::from("Service unavailable due to maintenance"));
    Ok(())
}

fn deserialize_path_string<'de, D>(deserializer: D) -> Result<Option<Vec<PathBuf>>, D::Error>
where
    D: Deserializer<'de>,
{
    struct VecStringVisitor;

    impl<'de> Visitor<'de> for VecStringVisitor {
        type Value = Option<Vec<PathBuf>>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a list of paths or a comma-separated path")
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            let mut vec = Vec::new();
            while let Some(value) = seq.next_element::<PathBuf>()? {
                vec.push(value);
            }
            Ok(Some(vec))
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Some(
                v.split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(PathBuf::from)
                    .collect(),
            ))
        }
    }

    deserializer.deserialize_any(VecStringVisitor)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PluginConfig {
    #[serde(default, deserialize_with = "lenient_bool")]
    enabled: bool,
    #[serde(default)]
    only_if: Option<String>,
    #[serde(default, deserialize_with = "deserialize_path_string")]
    content: Option<Vec<PathBuf>>,
}

fn lenient_bool<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    let value: String = Deserialize::deserialize(deserializer)?;
    if let Ok(val) = value.parse::<bool>() {
        return Ok(val);
    }
    match value.to_lowercase().as_str() {
        "true" | "yes" | "1" => Ok(true),
        "false" | "no" | "0" => Ok(false),
        _ => Err(serde::de::Error::custom("Invalid boolean value")),
    }
}

struct Plugin {
    filter: Option<Filter>,
    maintenance_pages: Option<MaintenancePages>,
}

impl PluginConfig {
    fn into_plugin(self) -> Option<Plugin> {
        if self.enabled {
            if let Some(only) = &self.only_if {
                info!("Plugin is enabled, only processing requests if: {only}");
            } else {
                info!("Plugin is enabled, processing all requests");
            }
            let filter = if let Some(filter) = &self.only_if {
                Some(match Filter::new(filter) {
                    Ok(f) => f,
                    Err(err) => {
                        error!("Failed to create filter: {err}");
                        return None;
                    }
                })
            } else {
                None
            };
            let maintenance_pages = self.content.map(MaintenancePages::new);
            Some(Plugin {
                filter,
                maintenance_pages,
            })
        } else {
            None
        }
    }
}

impl Guest for Plugin {
    fn handle_request(&self, request: Request, response: Response) -> (bool, i32) {
        let ctx = 0;
        let proceed = (true, ctx);
        if let Some(filter) = &self.filter {
            match filter.matches(&request) {
                Ok(true) => {
                    debug!("Request {:?} matches filter", request.uri().to_str());
                }
                Ok(false) => {
                    let uri = request.uri();
                    let uri = uri.to_str();
                    debug!("Request {uri:?} does not match filter, skipping...");
                    return proceed;
                }
                Err(err) => {
                    error!("Error matching request against filter: {err}");
                    // do not skip, show maintenance_pages
                }
            }
        }
        if let Some(pages) = &self.maintenance_pages {
            let accept_headers = request.header().values(&Bytes::from("Accept"));
            let accept_header = accept_headers
                .first()
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");
            match pages.send_page(accept_header, &response) {
                Ok(_) => {
                    debug!("Maintenance page sent successfully");
                }
                Err(err) => {
                    error!("Failed to send maintenance page: {err}");
                }
            };
        } else {
            debug!("No maintenance pages configured, proceeding with request");
            if let Err(e) = fallback(&response) {
                error!("Failed to send fallback response: {e}");
            }
        }
        (false, ctx)
    }
}

fn main() {
    http_wasm_guest::host::log::init().expect("Failed to initialize logging");
    let configuration = config();
    if configuration.is_empty() {
        info!("No plugin configuration found, skipping plugin registration");
        return;
    }
    let plugin_config = match serde_json::from_slice::<PluginConfig>(&configuration) {
        Ok(c) => c,
        Err(e) => {
            error!("Configuration: {configuration:?}");
            error!("Failed to parse plugin configuration: {e}");
            return;
        }
    };

    if let Some(plugin) = plugin_config.into_plugin() {
        register(plugin);
    }
}
