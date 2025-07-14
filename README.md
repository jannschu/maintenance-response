<h1 align="center">
  maintenance-response
</h1>

<p align="center">
  A <a href="https://traefik.io/traefik">Traefik</a> middleware for showing maintenance pages.
</p>

## Features

- Custom response content based on the `Accept` header
- Filtering requests using a [Cloudflare filter expression](https://github.com/cloudflare/wirefilter)
- WASM plugin, which runs in a sandbox
- End-To-End tests

## Installation example

Add the plugin to your `traefik.yml` configuration file:

```yaml
experimental:
  plugins:
    maintenance-response:
      moduleName: github.com/jannschu/maintenance-response
      version: v0.1.1
      settings:
        # because the plugin runs in a sandbox, you need to
        # mount your maintenance page/response content
        mounts:
          - /etc/traefik/maintenance:/maintenance:ro
```

Provide a dynamic configuration, e.g. file based:

```yaml
http:
  middlewares:
    maintenance:
      plugin:
        maintenance-response:
          # use this to enable or disable the plugin,
          # this configuration is dynamically reloaded by Traefik
          enabled: true
          # optional: specify response content,
          # the content is picked using the 'Accept' header
          # provided by the client
          responses:
            # must match the mount path
            - /maintenance/index.html
            - /maintenance/index.json
          # optional: specify a filter expression (see below)
          # for limiting the maintenance mode to specific requests
          onlyIf: |
            http.host == "example.com"
```

Then add the middleware to an entry point or service. For example, to add
it to an entry point:

```yaml
entryPoints:
  http:
    address: ":80"
    http:
      middlewares:
        - maintenance@file
```

## Filter expression

The plugin can be configured to only send the maintenance response for
certain requests. This is done using a filter expression based on
Cloudflare's Wirefilter syntax, which in turn is based on the
[Wireshark syntax](https://www.wireshark.org/docs/wsug_html_chunked/ChWorkBuildDisplayFilterSection.html).

### Examples

Limit maintenance mode to specific hosts:

```
http.host in { "example.com" "intern.example.com" }
```

Limit requests to paths:

```
not http.path ~ "^/admin/"
```

These examples can be combined to create more complex expressions.
We refer to the [Cloudflare documentation](https://developers.cloudflare.com/ruleset-engine/rules-language/operators/)
for more details on the syntax. Be aware that we do _not_
use the same field names ("variables") as Cloudflare.

### Supported fields

You may use the following fields in your filter expression:

- `http.host`: the HTTP host header
- `http.path`: the HTTP request path
- `http.method`: the HTTP request method (e.g. GET, POST)
- `http.ua`: the HTTP user agent header
- `src.ip`: the source IP address of the request
- `src.port`: the source port of the request
