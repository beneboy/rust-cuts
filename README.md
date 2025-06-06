# Rust Cuts

## Supercharged terminal commands with templating and context

**Stop fighting your shell history.** Turn complex, hard-to-remember commands into simple, reusable templates with interactive prompts.

## What Makes Rust Cuts Different

- 🎯 Guided parameter input means no more remembering syntax or editing long commands
- 🎨 Full context control gives each command its own working directory and environment  
- 📚 Instant fuzzy search finds any command in milliseconds, no matter how many you have
- 🔧 Templates include default values, descriptions, and validation
- ⚡ Modern terminal UX built for the way developers actually work today

The key insight: **most command-line tools assume you remember everything**. Rust Cuts assumes you don't, and that's perfectly fine.

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

## Real-World Examples

### 🐳 Docker Development Workflow

```yaml
- id: "docker-dev"
  description: "Start development environment with hot reload"
  command: ["docker", "compose", "-f", "docker-compose.{env}.yml", "up", "--build"]
  working_directory: "~/projects/{project}"
  parameters:
    - id: "project"
      default: "my-app"
      description: "Project directory name"
    - id: "env"
      default: "dev"
      description: "Environment (dev/staging/prod)"
  environment:
    COMPOSE_PROJECT_NAME: "{project}-{env}"

- id: "docker-logs"
  description: "View logs for a specific service"
  command: ["docker", "compose", "logs", "-f", "--tail=100", "{service}"]
  working_directory: "~/projects/{project}"
  parameters:
    - id: "project"
      default: "my-app"
    - id: "service"
      description: "Service name (web, db, redis, etc.)"
```

### 🚀 Kubernetes Operations

```yaml
- id: "k8s-deploy"
  description: "Deploy application to Kubernetes"
  command: ["kubectl", "apply", "-f", "k8s/{env}/", "--namespace", "{namespace}"]
  working_directory: "~/projects/{app}"
  parameters:
    - id: "app"
      default: "my-service"
    - id: "env"
      default: "staging"
      description: "Environment (dev/staging/prod)"
    - id: "namespace"
      default: "default"
  environment:
    KUBECONFIG: "~/.kube/config-{env}"

- id: "k8s-logs"
  description: "Stream logs from Kubernetes pods"
  command: ["kubectl", "logs", "-f", "deployment/{service}", "--namespace", "{namespace}"]
  parameters:
    - id: "service"
      description: "Service deployment name"
    - id: "namespace"
      default: "default"
  environment:
    KUBECONFIG: "~/.kube/config"
```

### 🗄️ Database Operations

```yaml
- id: "db-backup"
  description: "Backup database with timestamp"
  command: ["pg_dump", "-h", "{host}", "-U", "{user}", "-d", "{database}", "|", "gzip", ">", "backup_{database}_$(date +%Y%m%d_%H%M%S).sql.gz"]
  parameters:
    - id: "host"
      default: "localhost"
    - id: "user"
      default: "postgres"
    - id: "database"
      description: "Database name to backup"
  environment:
    PGPASSWORD: "{password}"

- id: "db-connect"
  description: "Connect to database with environment-specific credentials"
  command: ["psql", "-h", "{host}", "-U", "{user}", "-d", "{database}"]
  parameters:
    - id: "env"
      default: "dev"
      description: "Environment (dev/staging/prod)"
  environment:
    PGHOST: "db-{env}.company.com"
    PGUSER: "app_user"
    PGDATABASE: "myapp_{env}"
```

### 🔧 Development Tools

```yaml
- id: "test-watch"
  description: "Run tests in watch mode for specific module"
  command: ["cargo", "watch", "-x", "test {module} -- --nocapture"]
  working_directory: "~/rust-projects/{project}"
  parameters:
    - id: "project"
      default: "current-project"
    - id: "module"
      default: ""
      description: "Specific test module (optional)"

- id: "lint-fix"
  description: "Run linter and auto-fix issues"
  command: ["npm", "run", "lint:fix", "--", "--ext", ".{ext}", "src/"]
  working_directory: "~/projects/{project}"
  parameters:
    - id: "project"
      description: "Project directory"
    - id: "ext"
      default: "js,ts,jsx,tsx"
      description: "File extensions to lint"
  environment:
    NODE_ENV: "development"
```

### ☁️ AWS/Cloud Operations

```yaml
- id: "aws-deploy"
  description: "Deploy to AWS with specific profile and region"
  command: ["aws", "s3", "sync", "dist/", "s3://{bucket}/", "--delete", "--region", "{region}"]
  working_directory: "~/projects/{project}"
  parameters:
    - id: "project"
      description: "Project to deploy"
    - id: "bucket"
      description: "S3 bucket name"
    - id: "region"
      default: "us-east-1"
  environment:
    AWS_PROFILE: "{env}"
    AWS_DEFAULT_REGION: "{region}"

- id: "ssh-ec2"
  description: "SSH to EC2 instance by name tag"
  command: ["aws", "ec2", "describe-instances", "--filters", "Name=tag:Name,Values={name}", "--query", "'Instances[0].PublicIpAddress'", "--output", "text", "|", "xargs", "-I", "{}", "ssh", "-i", "~/.ssh/{key}.pem", "{user}@{}"]
  parameters:
    - id: "name"
      description: "EC2 instance name tag"
    - id: "user"
      default: "ubuntu"
    - id: "key"
      default: "my-aws-key"
  environment:
    AWS_PROFILE: "production"
```

### 🔄 Git Workflow Automation

```yaml
- id: "git-feature"
  description: "Create and switch to feature branch"
  command: ["git", "checkout", "-b", "feature/{ticket}-{description}", "develop"]
  working_directory: "~/projects/{project}"
  parameters:
    - id: "project"
      default: "current"
    - id: "ticket"
      description: "Ticket/issue number"
    - id: "description"
      description: "Brief feature description (use-dashes)"

- id: "git-release"
  description: "Create release branch and push"
  command: ["git", "checkout", "-b", "release/{version}", "develop", "&&", "git", "push", "-u", "origin", "release/{version}"]
  working_directory: "~/projects/{project}"
  parameters:
    - id: "project"
      default: "current"
    - id: "version"
      description: "Release version (e.g., 1.2.0)"
```

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

## The Problem with Static Commands

Most developers end up with something like this:

```bash
# Your .bashrc file grows and grows...
alias k8s-deploy-staging="kubectl apply -f k8s/staging/ --namespace staging"
alias k8s-deploy-prod="kubectl apply -f k8s/prod/ --namespace prod"  
alias k8s-deploy-dev="kubectl apply -f k8s/dev/ --namespace dev"
alias docker-logs-web="docker compose logs -f web"
alias docker-logs-db="docker compose logs -f db"
alias docker-logs-redis="docker compose logs -f redis"
# ...and 47 more variations you'll never remember
```

## The Rust Cuts Approach

Instead of memorizing dozens of static commands, define flexible templates:

```yaml
- id: "k8s-deploy"
  description: "Deploy to any environment"
  command: ["kubectl", "apply", "-f", "k8s/{env}/", "--namespace", "{env}"]
  parameters:
    - id: "env"
      description: "Environment (staging/prod/dev)"

- id: "docker-logs"
  description: "View logs for any service"  
  command: ["docker", "compose", "logs", "-f", "{service}"]
  parameters:
    - id: "service"
      description: "Service name"
```

Now `rc` gives you an interactive menu, and you just fill in the blanks. **One template handles infinite variations.**

## Advanced Usage

### Direct Command Execution
```bash
# Run specific command by ID
rc k8s-deploy

# With parameters (no prompts)
rc k8s-deploy -p env=staging

# Positional parameters  
rc docker-logs web

# Dry run to preview
rc --dry-run k8s-deploy

# Force execution (skip confirmation)
rc --force docker-logs web

# Rerun last command
rc --rerun-last-command
# or simply
rc -r
```

### Command Line Integration
```bash
# Use in scripts
if rc --dry-run deploy-prod | grep -q "production"; then
    echo "About to deploy to production!"
    rc --force deploy-prod
fi

# Chain with other commands
rc build-docker && rc deploy-staging && rc run-tests
```

# Development

## Running Subprojects

```bash
# Run the CLI tool
cargo run -p rust-cuts-cli

# Run the GUI (when enabled)  
cargo run -p rust-cuts-gui

# Run all tests
cargo test

# Check code quality
cargo clippy -- -D warnings
cargo fmt --check
```