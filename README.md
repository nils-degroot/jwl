# jwl

Program to create and view work logs using Jira

## Dependencies

The application requires `cargo` to build.

## Installation

To build the application:

```sh
cargo build --release
```

Then a binary can be found at `target/release/jwl`.

## Configuration

To generate a config, run the program with the config subcommand. This will
generate a config for you.

Some configuration is required. A configuration file should be created at one
of the following places:

- Linux: `$HOME/.config/jwl/config.yml`
- macOS: `$HOME/Libary/Application Support/jwl/config.yml`
- Windows: `%AppData%\Roaming\jwl\config.yml`

The configuration should be in yaml and should look as follows:

```yaml
jira_domain: { Domain pointing to jira, for example https://test.atlassian.net }
authorization:
  username: { Username }
  api_token: { Api token }
```

Access token authentication could be used as follows:

```yaml
authorization:
  access_token: { Access token }
```

### Contexts

It is possible to use multiple contexts. This allows you to read from multiple
Jira instances without having to update the config file each time. To do this,
the same config needs to be set up as an array, i.e:

```
- name: default
  jira_domain: { Domain pointing to jira, for example https://test.atlassian.net }
  authorization:
    username: { Username }
    api_token: { Api token }
```

Then for the read and view commands, a context name can be passed that should
match the given name in the config.
