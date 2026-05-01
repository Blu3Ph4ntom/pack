//! Rails-specific commands and helpers
//!
//! Pack provides first-class support for Rails developers with commands
//! that match their workflow and make development faster.

use anyhow::{Context, Result};
use std::io::ErrorKind;
use std::path::PathBuf;
use std::process::Command;

/// Rails application helpers
pub struct RailsApp {
    path: PathBuf,
}

impl RailsApp {
    /// Get the project path
    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    /// Discover a Rails app in the current directory
    pub fn discover() -> Result<Option<Self>> {
        let path = std::env::current_dir().context("failed to get current dir")?;
        let config = path.join("config");

        if config.exists() && config.is_dir() {
            // Check for Rails-specific files
            let application = config.join("application.rb");
            let routes = config.join("routes.rb");

            if application.exists() || routes.exists() {
                return Ok(Some(Self { path }));
            }
        }

        Ok(None)
    }

    /// Get the Rails environment (development, test, production)
    pub fn env(&self) -> String {
        std::env::var("RAILS_ENV").unwrap_or_else(|_| "development".to_string())
    }

    /// Check if this is a Rails 8+ app with Solid Queue/Cache
    pub fn is_rails_8(&self) -> bool {
        let gemfile = self.path.join("Gemfile");
        if let Ok(content) = std::fs::read_to_string(&gemfile) {
            content.contains("rails")
                && (content.contains("solid_queue")
                    || content.contains("solid_cache")
                    || content.contains("propshaft"))
        } else {
            false
        }
    }

    /// Run a Rails command
    pub fn run_rails_cmd(&self, cmd: &[&str]) -> Result<()> {
        let mut args = vec!["exec".to_string(), "rails".to_string()];
        args.extend(cmd.iter().map(|arg| arg.to_string()));
        let status = run_tool("bundle", &args, Some(&self.path)).context("Rails command failed")?;

        if !status.success() {
            anyhow::bail!("Rails command failed with exit code: {:?}", status.code());
        }

        Ok(())
    }

    /// Run a rake task
    pub fn run_rake(&self, task: &str) -> Result<()> {
        let status = run_tool(
            "bundle",
            &["exec".to_string(), "rake".to_string(), task.to_string()],
            Some(&self.path),
        )
        .context("Rake task failed")?;

        if !status.success() {
            anyhow::bail!("Rake task failed with exit code: {:?}", status.code());
        }

        Ok(())
    }

    /// Start the Rails server
    pub fn server(&self, port: Option<u16>, detached: bool) -> Result<()> {
        let mut args = vec![
            "exec".to_string(),
            "rails".to_string(),
            "server".to_string(),
            "-b".to_string(),
            "0.0.0.0".to_string(),
        ];

        if let Some(p) = port {
            args.push("-p".to_string());
            args.push(p.to_string());
        }

        if detached {
            spawn_tool("bundle", &args, Some(&self.path))?;
            println!(
                "Rails server started in background on port {}",
                port.unwrap_or(3000)
            );
        } else {
            let status =
                run_tool("bundle", &args, Some(&self.path)).context("Rails server failed")?;
            if !status.success() {
                anyhow::bail!("Rails server exited with code: {:?}", status.code());
            }
        }

        Ok(())
    }

    /// Open Rails console
    pub fn console(&self) -> Result<()> {
        run_tool(
            "bundle",
            &[
                "exec".to_string(),
                "rails".to_string(),
                "console".to_string(),
            ],
            Some(&self.path),
        )
        .context("Rails console failed")?;

        Ok(())
    }

    /// Run tests
    pub fn test(&self, args: &[String]) -> Result<()> {
        let mut command_args = vec!["exec".to_string(), "rails".to_string(), "test".to_string()];
        command_args.extend(args.iter().cloned());
        let status = run_tool("bundle", &command_args, Some(&self.path)).context("Tests failed")?;

        if !status.success() {
            anyhow::bail!("Tests failed with exit code: {:?}", status.code());
        }

        Ok(())
    }

    /// Run RSpec tests
    pub fn rspec(&self, args: &[String]) -> Result<()> {
        let mut command_args = vec!["exec".to_string(), "rspec".to_string()];
        command_args.extend(args.iter().cloned());
        let status = run_tool("bundle", &command_args, Some(&self.path)).context("RSpec failed")?;

        if !status.success() {
            anyhow::bail!("RSpec failed with exit code: {:?}", status.code());
        }

        Ok(())
    }

    /// Database operations
    pub fn db(&self, operation: &str) -> Result<()> {
        match operation {
            "create" => self.run_rails_cmd(&["db:create"]),
            "drop" => self.run_rails_cmd(&["db:drop"]),
            "migrate" => self.run_rails_cmd(&["db:migrate"]),
            "rollback" => self.run_rails_cmd(&["db:rollback"]),
            "seed" => self.run_rails_cmd(&["db:seed"]),
            "reset" => self.run_rails_cmd(&["db:reset"]),
            "setup" => self.run_rails_cmd(&["db:setup"]),
            "schema:load" => self.run_rails_cmd(&["db:schema:load"]),
            "migrate:status" => self.run_rails_cmd(&["db:migrate:status"]),
            _ => anyhow::bail!("Unknown db operation: {}. Try: create, drop, migrate, rollback, seed, reset, setup", operation),
        }
    }

    /// Asset pipeline
    pub fn assets(&self, operation: &str) -> Result<()> {
        match operation {
            "precompile" => self.run_rake("assets:precompile"),
            "clean" => self.run_rake("assets:clean"),
            "clobber" => self.run_rake("assets:clobber"),
            _ => anyhow::bail!(
                "Unknown assets operation: {}. Try: precompile, clean, clobber",
                operation
            ),
        }
    }

    /// Cache operations
    pub fn cache(&self, operation: &str) -> Result<()> {
        match operation {
            "clear" => self.run_rails_cmd(&["cache:clear"]),
            "warm" => self.run_rails_cmd(&["cache:warm"]),
            _ => anyhow::bail!("Unknown cache operation: {}. Try: clear, warm", operation),
        }
    }

    /// Generate a Docker setup optimized for pack
    pub fn generate_docker(&self) -> Result<PathBuf> {
        let dockerfile = self.path.join("Dockerfile.pack");
        let compose = self.path.join("docker-compose.pack.yml");

        let dockerfile_content = r#"# Multi-stage Dockerfile for Rails with Pack
# Uses Pack for RubyGems-compatible project workflows.

FROM ruby:3.3-slim AS base
WORKDIR /app
RUN apt-get update -qq && apt-get install -y --no-install-recommends \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Install Pack
RUN curl -fsSL https://raw.githubusercontent.com/Blu3Ph4ntom/pack/main/scripts/install.sh | bash
ENV PATH="/usr/local/bin:$PATH"

# Install Node.js for asset pipeline
FROM base AS node
RUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y nodejs \
    && rm -rf /var/lib/apt/lists/*

# Install PostgreSQL client
FROM node AS postgres
RUN apt-get update -qq && apt-get install -y --no-install-recommends \
    postgresql-client \
    && rm -rf /var/lib/apt/lists/*

# Final stage
FROM postgres AS production
RUN groupadd --gid 1000 rails && \
    useradd --uid 1000 --gid rails --shell /bin/bash --create-home rails

# Copy Gemfile and install gems
COPY --chown=rails:rails Gemfile Gemfile.lock ./
RUN bundle config set --local deployment 'true' && \
    bundle config set --local without 'development test' && \
    bundle install --jobs 4 --retry 3

# Copy application
COPY --chown=rails:rails . .

# Install JavaScript dependencies
RUN if [ -f "package.json" ]; then npm install; fi

# Precompile assets
RUN bundle exec rake assets:precompile

# Expose port
EXPOSE 3000

# Start server
CMD ["bundle", "exec", "rails", "server", "-b", "0.0.0.0"]

# Development stage
FROM postgres AS development
RUN bundle config set --local path 'vendor/bundle'

COPY --chown=rails:rails Gemfile Gemfile.lock ./
RUN bundle install --jobs 4 --retry 3

COPY --chown=rails:rails . .

RUN if [ -f "package.json" ]; then npm install; fi

EXPOSE 3000

CMD ["bundle", "exec", "rails", "server", "-b", "0.0.0.0"]
"#;

        let compose_content = r#"# Docker Compose for Rails with Pack
# Use: docker compose -f docker-compose.pack.yml up

services:
  db:
    image: postgres:16-alpine
    environment:
      POSTGRES_PASSWORD: password
    volumes:
      - postgres_data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U postgres"]
      interval: 5s
      timeout: 5s
      retries: 5

  redis:
    image: redis:7-alpine
    volumes:
      - redis_data:/data

  app:
    build:
      context: .
      dockerfile: Dockerfile.pack
      target: development
    command: bash -c "rm -f tmp/pids/server.pid && bundle exec rails server -b 0.0.0.0"
    volumes:
      - .:/app
      - bundle_cache:/usr/local/bundle
    ports:
      - "3000:3000"
    depends_on:
      db:
        condition: service_healthy
      redis:
        condition: service_started
    environment:
      DATABASE_URL: postgres://postgres:password@db:5432/app_development
      REDIS_URL: redis://redis:6379/1
      RAILS_ENV: development

volumes:
  postgres_data:
  redis_data:
  bundle_cache:
"#;

        std::fs::write(&dockerfile, dockerfile_content)
            .context("failed to write Dockerfile.pack")?;

        std::fs::write(&compose, compose_content)
            .context("failed to write docker-compose.pack.yml")?;

        println!("Generated:");
        println!("  - Dockerfile.pack (multi-stage, optimized for pack)");
        println!("  - docker-compose.pack.yml (development + production)");
        println!();
        println!("To use:");
        println!("  docker compose -f docker-compose.pack.yml build app");
        println!("  docker compose -f docker-compose.pack.yml up app");

        Ok(dockerfile)
    }

    /// Generate a .packignore file (like .dockerignore)
    #[allow(dead_code)]
    pub fn generate_packignore(&self) -> Result<PathBuf> {
        let packignore = self.path.join(".packignore");
        let content = r#"# Pack ignore - gems to not cache
# Similar to .dockerignore for faster installs

# Ruby version files we don't need cached
.ruby-version
.ruby-gemset

# Documentation
*.md
docs/

# Test fixtures
spec/fixtures/
test/fixtures/

# Log files
log/*.log
tmp/

# System files
.DS_Store
Thumbs.db

# IDE
.idea/
.vscode/
*.swp
*.swo

# Node (if using asset pipeline)
node_modules/
"#;

        std::fs::write(&packignore, content).context("failed to write .packignore")?;

        println!("Generated .packignore");
        println!("This tells pack which files to skip when caching gems.");

        Ok(packignore)
    }

    /// Check for Rails-specific issues
    pub fn doctor(&self) -> Result<Vec<String>> {
        let mut issues = Vec::new();

        // Check Rails version
        let gemfile = self.path.join("Gemfile");
        if let Ok(content) = std::fs::read_to_string(&gemfile) {
            // Check for outdated Rails
            if content.contains("gem 'rails'") && !content.contains("~> 8") {
                issues.push("Rails 7.x or older detected. Consider upgrading to Rails 8 for Solid Queue/Cache support.".to_string());
            }

            // Check for old asset pipeline
            if content.contains("sassc-rails") {
                issues.push(
                    "sassc-rails detected. Consider migrating to Propshaft (Rails 8 default)."
                        .to_string(),
                );
            }

            // Check for Sprockets vs Propshaft
            if content.contains("sprockets-rails") && !content.contains("propshaft") {
                issues.push(
                    "Sprockets detected. Propshaft is faster and the Rails 8 default.".to_string(),
                );
            }
        }

        // Check database config
        let database_yml = self.path.join("config").join("database.yml");
        if !database_yml.exists() {
            issues.push("config/database.yml missing. Run: rails db:create db:migrate".to_string());
        }

        // Check for missing secret key
        if self.env() == "production" {
            if std::env::var("SECRET_KEY_BASE").is_err() {
                issues.push("SECRET_KEY_BASE not set. Run: rails credentials:edit".to_string());
            }
        }

        // Check Solid Queue / Solid Cache setup (Rails 8)
        if self.is_rails_8() {
            let solid_queue = self.path.join("config").join("queue.yml");
            if !solid_queue.exists() {
                issues.push("Solid Queue not configured (config/queue.yml missing). Run: bin/rails generate solid_queue:install".to_string());
            }
        }

        Ok(issues)
    }
}

/// Initialize a new Rails project with pack
#[allow(dead_code)]
pub fn rails_new(project_name: &str, args: &[String]) -> Result<PathBuf> {
    println!("Creating new Rails project: {}", project_name);

    let project_path = PathBuf::from(".").join(project_name);
    std::fs::create_dir_all(&project_path)?;

    // Use rails new but with pack-friendly options
    let mut cmd = Command::new("rails");
    cmd.arg("new");
    cmd.arg(&project_path);
    cmd.arg("--skip-bundle"); // We'll use pack instead

    // Add any additional args
    for arg in args {
        cmd.arg(arg);
    }

    println!("Running: rails new {} --skip-bundle", project_name);
    let status = cmd.status().context("rails new failed")?;

    if !status.success() {
        anyhow::bail!("rails new failed with exit code: {:?}", status.code());
    }

    // Now set up pack for this project
    println!();
    println!("Setting up pack for the project...");

    std::env::set_current_dir(&project_path)?;

    // Generate Gemfile.lock with bundler first (we'll create pack.lock later)
    let status = Command::new("bundle")
        .args(["install"])
        .current_dir(&project_path)
        .status()
        .context("bundle install failed")?;

    if !status.success() {
        println!("Warning: bundle install had issues, but you can use pack instead");
    }

    // Generate pack-specific files
    let app = RailsApp::discover()?.unwrap();
    app.generate_packignore()?;

    if args.contains(&"--docker".to_string()) {
        app.generate_docker()?;
    }

    println!();
    println!("Done! Your Rails project is ready.");
    println!();
    println!("Next steps:");
    println!("  cd {}", project_name);
    println!("  pack install     # Install project gems");
    println!("  pack lock        # Generate pack.lock");
    println!("  pack server      # Start Rails server");

    if args.contains(&"--docker".to_string()) {
        println!();
        println!("Docker support:");
        println!("  docker compose -f docker-compose.pack.yml up");
    }

    Ok(project_path)
}

fn run_tool(
    tool: &str,
    args: &[String],
    current_dir: Option<&PathBuf>,
) -> Result<std::process::ExitStatus> {
    let mut cmd = Command::new(tool);
    cmd.args(args);
    if let Some(dir) = current_dir {
        cmd.current_dir(dir);
    }

    match cmd.status() {
        Ok(status) => Ok(status),
        Err(e) if e.kind() == ErrorKind::NotFound => {
            let mut fallback = Command::new("ruby");
            fallback.arg("-S").arg(tool).args(args);
            if let Some(dir) = current_dir {
                fallback.current_dir(dir);
            }
            fallback.status().with_context(|| {
                format!("failed to run `{}` and fallback `ruby -S {}`", tool, tool)
            })
        }
        Err(e) => Err(e).with_context(|| format!("failed to run `{}`", tool)),
    }
}

fn spawn_tool(tool: &str, args: &[String], current_dir: Option<&PathBuf>) -> Result<()> {
    let mut cmd = Command::new(tool);
    cmd.args(args);
    if let Some(dir) = current_dir {
        cmd.current_dir(dir);
    }

    match cmd.spawn() {
        Ok(_) => Ok(()),
        Err(e) if e.kind() == ErrorKind::NotFound => {
            let mut fallback = Command::new("ruby");
            fallback.arg("-S").arg(tool).args(args);
            if let Some(dir) = current_dir {
                fallback.current_dir(dir);
            }
            fallback.spawn().map(|_| ()).with_context(|| {
                format!("failed to spawn `{}` and fallback `ruby -S {}`", tool, tool)
            })
        }
        Err(e) => Err(e).with_context(|| format!("failed to spawn `{}`", tool)),
    }
}
