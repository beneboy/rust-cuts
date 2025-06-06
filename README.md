# Rust Cuts

## Supercharged terminal commands with templating and context

## Why Rust Cuts?

- **Stop fighting your shell history.** Run complex commands with a single keystroke
- **Templated commands with prompts.** No more editing long commands by hand
- **Context-aware execution.** Automatically set working directory and environment variables

## What?

Supercharged terminal aliases.

Save a list of named commands, and run them with `rc`.
No need to hit Ctrl-R and spend ages scrolling through your history to remember the command you need.
Easier than creating aliases because you can template the command with values that are be prompted for,
as well as specifying a working directory or environment variables to always use for the command.

## Setup

Rust and build tools (`cargo`) must be installed. Get these from [rustup.rs](https://rustup.rs)

Clone this repository and build/install from source with:

```shell
$ cargo install --path .
```

Next create the directory `~/.rust-cuts/` and definition YAML file `~/.rust-cuts/commands.yml`.
See [sample-commands.yml](./sample-commands.yml) for an example.

## Simple Example

Basic *Hello World* example:

```yaml
- name: "Do hello world!"
  command: ["echo", "Hello world!"]
```

When executing, a list of commands is displayed.
These can be scrolled through with cursor keys or mousewheel.
Hit `<enter>` to execute the selected command.

Commands can also be clicked on.

## Templates

Template tokens are specified inside braces `{}`.

In this example, we want to SSH to different AWS EC2 instances that have been created,
all which use the shared SSH key stored in AWS,
but have different usernames and hosts (as they are created).

The username and host portion of the command is templated.

This is added to the `commands.yml`:

```yaml
- name: "Do hello world!"
  command: ["echo", "Hello world!"]
- name: "SSH to EC2"
  command: ["ssh", "-i", "~/path/to/aws-key.pem", "{username}@{host}"]
```

After selecting the command, the parameters are prompted for.

```shell
Please give value for `host`: 10.1.2.3
Please give value for `username`: ec2-user
Executing command:
ssh -i ~/path/to/aws-key.pem ec2-user@10.1.2.3
Are you sure you want to run? ([Y]es/[n]o/[c]hange parameters): y
… SSH session starts…
```

### Defaults for parameters

Specify a list of `parameters` for a command, each with a `name` and `default`.
The `name` must match the value for a template token.
If no input is given when prompted, the default will be used.

We can update the previous example to provide a default for `username`:

```yaml
- name: "SSH to EC2"
  command: ["ssh", "-i", "~/path/to/aws-key.pem", "{username}@{host}"]
  parameters:
    - name: "username"
      default: "ubuntu"
```
Now, if no value is provided, it defaults to `ubuntu`:

```shell
Please give value for `host`: 10.1.2.3
Please give value for `username` [ubuntu]:
Executing command:
ssh -i ~/path/to/aws-key.pem ubuntu@10.1.2.3
Are you sure you want to run? ([Y]es/[n]o/[c]hange parameters): y
… SSH session starts…
```

## Working Directory

Specify a `working_directory` for command to change into that directory before executing.
All paths will now be relative to that directory.

In this example, `make build` is executed inside a project directory.

```yaml
- name: "Build project"
  command: ["make", "build"]
  working_directory: "~/projects/rust-cuts/"
```

`rc` executes the command and returns to the original directory afterward.

## Environment Variables

Environment variables are specified in the `environment` dictionary for the command,
as key-value pairs.
For example, if there's a command that should always be run as a specific AWS profile,
it can be specified as an environment variable.

This example lists objects in an S3 bucket,
using the `aws` CLI tool.
The AWS profile environment variable `AWS_PROFILE` is always set to `dev`, 
so the `dev` profile is used for authentication.

```yaml
- name: "List objects in S3 bucket"
  command: ["aws", "s3", "ls", "{bucket}"]
  environment:
    AWS_PROFILE: dev
```

## Rerun Last Command

To rerun the previous command, type `r` at the command list.
You have the opportunity to enter new values for parameters.

Or, to automatically select the last command, execute `rc -r`.
You will be prompted to confirm the command to run,
and will have the chance to update parameters.

To force run the last command, without confirming or changing parameters,
execute with the `force` flag as well, or `rc -rf`.

## Execution in Shell

Commands are executed inside your shell,
so normal aliases and shell expansion is available.

Commands are not escaped, so shell injection is possible. This is by design, but may change.

You should escape values that are prompted. In this simple example the command `cat`s a file:

```yaml
- name: "cat a file"
  command: ["cat", "{path}"]
```

When entering `path`, either quote the input or use backslashes before spaces.

Here is how various inputs will behave.

This first example has space separated values:

```shell
Please give value for `path`: file1.txt file2.txt
```

This will cat `file1.txt` and `file2.txt`.

By using quotes a path with spaces can be used:

```shell
Please give value for `path`: "file with spaces.txt"
```

Similarly, backslashes can be used:

```shell
Please give value for `path`: file\ with\ spaces.txt
```

Both of these will cat the file `file with spaces.txt`.


## Adding Colors To Commands

To help differentiate between commands as they are listed,
the background and foreground colors can be set.
Add a `metadata` item under the command, with `foreground_color` and/or `background_color` options.

For example:

```yaml
- name: "This command is colored!"
  command: ["echo", "So pretty!"]
  metadata:
    background_color:
      name: white  # named color
    foreground_color:
      name: red
```

Colors can be specified using RGB, ANSI value, or name.

```yaml
- name: "This command is colored!"
  command: ["echo", "So pretty!"]
  metadata:
    background_color:
      rgb: [255, 255, 255]  # white color using RGB components, 0 to 255
    foreground_color:
      name: [255, 0, 0]  # red color
```

(RGB colors are not supported on all platforms, e.g. macOS's built-in Terminal).

```yaml
- name: "This command is colored!"
  command: ["echo", "So pretty!"]
  metadata:
    background_color:
      ansi: 255  # white color using ANSI code
    foreground_color:
      ansi: 9  # red color
```

Both `background_color` and `foreground_color` are optional (you may choose to specify only one).
Each does not need to use the same mode,
for example you may use `ansi` for foreground and `rgb` for background.

Only one of `ansi`, `rgb` or `name` may be specified per color.
Specifying more than one will cause an error at runtime.

### Color Names

The following named colors are supported.
They are case-insensitive.

- `Black`
- `DarkGrey`
- `Red`
- `DarkRed`
- `Green`
- `DarkGreen`
- `Yellow`
- `DarkYellow`
- `Blue`
- `DarkBlue`
- `Magenta`
- `DarkMagenta`
- `Cyan`
- `DarkCyan`
- `White`
- `Grey`

# Running Subprojects

GUI: cargo run -p rust-cuts-gui
CLI: cargo run -p rust-cuts-cli