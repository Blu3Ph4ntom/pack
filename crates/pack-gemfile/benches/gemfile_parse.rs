use criterion::{black_box, criterion_group, criterion_main, Criterion};
use pack_gemfile::parse_gemfile;
use pack_gemfile::lockfile::load_lockfile;
use tempfile::TempDir;

fn gemfile_100_deps() -> String {
    let mut content = String::from("source 'https://rubygems.org'\n\n");
    let gems = [
        "rails", "rake", "rspec", "nokogiri", "puma", "sidekiq", "devise", "kaminari",
        "carrierwave", "paperclip", "Draper", "friendly_id", "ancestry", "paranoia",
        "audit", "cocoon", "devise_token_auth", "omniauth", "doorkeeper",
        "cancancan", "pundit", "rolify", "bcrypt", "jwt", "oauth2", "omniauth-facebook",
        "omniauth-github", "linkedin", "twitter", "facebook", "google_oauth2",
        "rack-attack", "rack-cors", "rack-mini-profiler", "bullet", "flamegraph", "stackprof",
        "rubocop", "rspec", "factory_bot", "faker", "factory_bot_rails", "shoulda_matcher",
        "database_cleaner", "simplecov", "coveralls", "brakeman", "bundler-audit",
        "gemnasium", "travis", "license_finder", "overcommit", "hirb", "pry", "pry-byebug",
        "pry-rails", "pry-doc", "awesome_print", "terminal-table", "colorize",
        "kramdown", "redcarpet", "rouge", "pygments", "github-markup", "asciidoctor",
        "liquid", "tilt", "slim", "haml", "erb", "erubis", "temple",
        "markaby", "bson", "mongo", "mongoid", "moped", "redis", "redis-namespace",
        "resque", "sidekiq", "solid_queue", "good_job", "delayed_job", "backburner",
        "que", "qc", "sucker_punch", "async", "concurrent-ruby", "eventmachine",
    ];
    for gem in gems {
        content.push_str(&format!("gem '{}'\n", gem));
    }
    content
}

fn parse_gemfile_benchmark(c: &mut Criterion) {
    let content = gemfile_100_deps();

    c.bench_function("parse_gemfile_100_deps", |b| {
        b.iter(|| parse_gemfile(black_box(&content)))
    });
}

fn lockfile_50_gems_content() -> String {
    let mut content = String::from("GEM
  remote: https://rubygems.org/
  specs:
");
    let gems: Vec<(&str, &str)> = vec![
        ("rails", "7.1.0"), ("actionview", "7.1.0"), ("actionpack", "7.1.0"),
        ("activerecord", "7.1.0"), ("activejob", "7.1.0"), ("railties", "7.1.0"),
        ("activesupport", "7.1.0"), ("bundler", "2.4.0"), ("rake", "13.0.0"),
        ("rspec", "3.12.0"), ("rspec-core", "3.12.0"), ("rspec-expectations", "3.12.0"),
        ("rspec-mocks", "3.12.0"), ("rspec-support", "3.12.0"), ("nokogiri", "1.15.0"),
        ("puma", "6.3.0"), ("rack", "2.2.0"), ("concurrent-ruby", "1.2.0"),
        ("i18n", "1.14.0"), ("minitest", "5.18.0"), ("tzinfo", "2.0.6"),
        ("zeitwerk", "2.6.0"), ("sprockets", "4.2.0"), ("sprockets-rails", "3.4.0"),
        ("thor", "1.2.0"), ("yard", "0.9.0"), ("coderay", "1.1.0"),
        ("diff-lcs", "1.5.0"), ("erubis", "2.7.0"), ("liquid", "5.4.0"),
    ];
    for (name, version) in &gems {
        content.push_str(&format!("    {} ({})\n", name, version));
    }
    content.push_str("\nPLATFORMS\n  ruby\n\nDEPENDENCIES\n");
    for (name, _) in &gems[..20] {
        content.push_str(&format!("  {}\n", name));
    }
    content.push_str("
BUNDLED WITH
   2.4.0
");
    content
}

fn parse_lockfile_benchmark(c: &mut Criterion) {
    let content = lockfile_50_gems_content();
    let temp_dir = TempDir::new().unwrap();
    let lockfile_path = temp_dir.path().join("Gemfile.lock");
    std::fs::write(&lockfile_path, &content).unwrap();

    c.bench_function("parse_lockfile_50_gems", |b| {
        b.iter(|| load_lockfile(black_box(&lockfile_path)))
    });
}

criterion_group!(benches, parse_gemfile_benchmark, parse_lockfile_benchmark);
criterion_main!(benches);
