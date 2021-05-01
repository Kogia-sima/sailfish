# Configuration

## Derive options

You can control the rendering behaviour via `template` attribute.

``` rust
#[derive(TemplateOnce)]
#[template(path = "template.stpl", escape = false)]
struct TemplateStruct {
    ...
}
```

`template` attribute accepts the following options.

- `path`: path to template file. This options is always required.
- `escape`: Enable HTML escaping (default: `true`)
- `delimiter`: Replace the '%' character used for the tag delimiter (default: '%')
- `rm_whitespace`: try to strip whitespaces as much as possible without collapsing HTML structure (default: `false`). This option might not work correctly if your templates have inline `script` tag.

You can split the options into multiple `template` attributes.

``` rust
#[derive(TemplateOnce)]
#[template(path = "template.stpl")]
#[template(delimiter = '?')]
#[template(rm_whitespace = true)]
struct TemplateStruct {
    ...
}
```

## Configuration file

Sailfish allows global and local configuration in a file named `sailfish.toml`. Sailfish looks for this file in same directory as `Cargo.toml` and all parent directories.
If, for example, `Cargo.toml` exists in `/foo/bar/baz` directory, then the following configuration files would be scanned in this order.

- `/foo/bar/baz/sailfish.toml`
- `/foo/bar/sailfish.toml`
- `/foo/sailfish.toml`
- `/sailfish.toml`

If a key is specified in multiple configuration files, the value in the deeper directory takes precedence over ancestor directories.

If a key is specified in both configuration file and derive options, then the value specified in the derive options takes precedence over the configuration file.

### Configuration file format

Configuration files are written in the TOML 0.5 format. Here is the default configuration:

``` toml
template_dirs = ["templates"]
escape = true
delimiter = "%"

[optimizations]
rm_whitespace = false
```

You can specify another template directory in `template_dirs` option. Other options are same as derive options.

You can also embed environment variables in `template_dirs` paths by wrapping the variable name with `${` and `}` like `${MY_ENV_VAR}`:

```toml
template_dirs = ["${CI}/path/to/project/${MYVAR}/templates"]
```
