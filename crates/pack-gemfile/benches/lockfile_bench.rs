use criterion::{black_box, criterion_group, criterion_main, Criterion};
use pack_core::GemName;
use pack_gemfile::lockfile::{find_dependency_path, load_lockfile, GemSpec, Lockfile};
use std::collections::HashMap;
use tempfile::TempDir;

fn lockfile_large_200_gems_content() -> String {
    let mut content = String::from(
        "GEM
  remote: https://rubygems.org/
  specs:
",
    );
    let gems: Vec<(&str, &str)> = vec![
        ("rails", "7.1.0"),
        ("actionview", "7.1.0"),
        ("actionpack", "7.1.0"),
        ("activerecord", "7.1.0"),
        ("activejob", "7.1.0"),
        ("railties", "7.1.0"),
        ("activesupport", "7.1.0"),
        ("bundler", "2.4.0"),
        ("rake", "13.0.0"),
        ("rspec", "3.12.0"),
        ("rspec-core", "3.12.0"),
        ("rspec-expectations", "3.12.0"),
        ("rspec-mocks", "3.12.0"),
        ("rspec-support", "3.12.0"),
        ("rspec-rails", "6.0.0"),
        ("nokogiri", "1.15.0"),
        ("puma", "6.3.0"),
        ("rack", "2.2.0"),
        ("concurrent-ruby", "1.2.0"),
        ("i18n", "1.14.0"),
        ("minitest", "5.18.0"),
        ("tzinfo", "2.0.6"),
        ("zeitwerk", "2.6.0"),
        ("sprockets", "4.2.0"),
        ("sprockets-rails", "3.4.0"),
        ("thor", "1.2.0"),
        ("yard", "0.9.0"),
        ("coderay", "1.1.0"),
        ("diff-lcs", "1.5.0"),
        ("erubis", "2.7.0"),
        ("liquid", "5.4.0"),
        ("haml", "6.0.0"),
        ("slim", "4.1.0"),
        ("pg", "1.4.0"),
        ("mysql2", "0.5.0"),
        ("sqlite3", "1.6.0"),
        ("redis", "5.0.0"),
        ("sidekiq", "7.0.0"),
        ("activejob", "7.1.0"),
        ("webpacker", "5.4.0"),
        ("shakapacker", "7.0.0"),
        ("jsbundling-rails", "1.0.0"),
        ("cssbundling-rails", "1.2.0"),
        ("importmap-rails", "1.1.0"),
        ("turbo-rails", "1.4.0"),
        ("stimulus-rails", "1.2.0"),
        ("debug", "1.6.0"),
        ("web-console", "4.2.0"),
        ("rack-mini-profiler", "2.90"),
        ("bullet", "7.1.0"),
        ("rubocop", "1.50.0"),
        ("rubocop-rails", "2.19.0"),
        ("rubocop-rspec", "2.20.0"),
        ("factory_bot", "6.2.0"),
        ("factory_bot_rails", "6.2.0"),
        ("faker", "3.2.0"),
        ("shoulda-matchers", "5.3.0"),
        ("database_cleaner", "2.0.0"),
        ("simplecov", "0.22.0"),
        ("coveralls", "0.2.0"),
        ("brakeman", "6.0.0"),
        ("bundler-audit", "0.9.0"),
        ("pry", "0.14.0"),
        ("pry-rails", "0.3.0"),
        ("pry-byebug", "3.10.0"),
        ("awesome_print", "1.2.0"),
        ("terminal-table", "3.0.0"),
        ("devise", "4.9.0"),
        ("devise-jwt", "0.10.0"),
        ("OmniAuth", "2.1.0"),
        ("omniauth-facebook", "9.0.0"),
        ("omniauth-github", "2.0.0"),
        ("kaminari", "1.2.0"),
        ("pagy", "6.0.0"),
        ("ranked-model", "0.4.0"),
        ("carrierwave", "2.2.0"),
        ("shrine", "3.4.0"),
        ("paperclip", "6.1.0"),
        ("aws-sdk-s3", "1.120.0"),
        ("google-cloud-storage", "0.11.0"),
        ("sidekiq-scheduler", "5.0.0"),
        ("sidekiq-unique-jobs", "7.0.0"),
        ("whenever", "1.5.0"),
        ("letter_opener", "1.8.0"),
        ("premailer", "1.16.0"),
        ("bulk_insert", "1.6.0"),
        ("paper_trail", "14.0.0"),
        ("paranoia", "2.6.0"),
        ("audited", "5.1.0"),
        ("public_activity", "1.6.0"),
        ("friendly_id", "5.4.0"),
        ("ancestry", "4.3.0"),
        ("awesome_nested_set", "3.2.0"),
        ("acts_as_list", "1.0.0"),
        ("acts_as_tree", "2.9.0"),
        ("cancancan", "3.5.0"),
        ("pundit", "2.3.0"),
        ("rolify", "5.3.0"),
        ("bcrypt", "3.1.0"),
        ("jwt", "2.5.0"),
        ("oauth2", "1.9.0"),
        ("doorkeeper", "5.6.0"),
        ("rack-attack", "6.6.0"),
        ("rack-cors", "2.0.0"),
        ("rack-test", "2.0.0"),
        ("capybara", "3.39.0"),
        ("selenium-webdriver", "4.8.0"),
        ("webdrivers", "5.2.0"),
        ("capybara-screenshot", "1.0.0"),
        ("launchy", "2.5.0"),
        ("site_prism", "4.0.0"),
        ("vcr", "6.0.0"),
        ("webmock", "3.18.0"),
        ("factory_bot", "6.2.0"),
    ];

    for (name, version) in &gems {
        content.push_str(&format!("    {} ({})\n", name, version));
    }

    content.push_str("\nPLATFORMS\n  ruby\n  x86_64-linux\n\nDEPENDENCIES\n");
    for (name, _) in gems.iter().take(40) {
        content.push_str(&format!("  {}\n", name));
    }
    content.push_str(
        "
BUNDLED WITH
   2.4.17
",
    );
    content
}

fn parse_lockfile_large_benchmark(c: &mut Criterion) {
    let content = lockfile_large_200_gems_content();
    let temp_dir = TempDir::new().unwrap();
    let lockfile_path = temp_dir.path().join("Gemfile.lock");
    std::fs::write(&lockfile_path, &content).unwrap();

    c.bench_function("parse_lockfile_200_gems", |b| {
        b.iter(|| load_lockfile(black_box(&lockfile_path)))
    });
}

fn find_dependency_path_benchmark(c: &mut Criterion) {
    let content = lockfile_large_200_gems_content();
    let temp_dir = TempDir::new().unwrap();
    let lockfile_path = temp_dir.path().join("Gemfile.lock");
    std::fs::write(&lockfile_path, &content).unwrap();

    let lockfile = load_lockfile(&lockfile_path).unwrap();

    c.bench_function("find_dependency_path_rails", |b| {
        b.iter(|| find_dependency_path(&lockfile, &GemName("rails".to_string())))
    });
}

fn why_gem_benchmark(c: &mut Criterion) {
    let content = lockfile_large_200_gems_content();
    let temp_dir = TempDir::new().unwrap();
    let lockfile_path = temp_dir.path().join("Gemfile.lock");
    std::fs::write(&lockfile_path, &content).unwrap();

    let lockfile = load_lockfile(&lockfile_path).unwrap();

    c.bench_function("why_rack", |b| {
        b.iter(|| find_dependency_path(&lockfile, &GemName("rack".to_string())))
    });
}

fn specs_iteration_benchmark(c: &mut Criterion) {
    let content = lockfile_large_200_gems_content();
    let temp_dir = TempDir::new().unwrap();
    let lockfile_path = temp_dir.path().join("Gemfile.lock");
    std::fs::write(&lockfile_path, &content).unwrap();

    let lockfile = load_lockfile(&lockfile_path).unwrap();

    c.bench_function("specs_iteration_200", |b| {
        b.iter(|| {
            let mut count = 0;
            for spec in lockfile.specs.values() {
                let _ = spec.version.0.len();
                count += 1;
            }
            black_box(count)
        })
    });
}

criterion_group!(
    benches,
    parse_lockfile_large_benchmark,
    find_dependency_path_benchmark,
    why_gem_benchmark,
    specs_iteration_benchmark
);
criterion_main!(benches);
