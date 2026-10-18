use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

#[derive(Debug, Deserialize)]
struct Naming {
    repo_name_pattern: String,
    domain_name_pattern: String,
}

#[derive(Debug, Deserialize)]
struct CacheConfig {
    index_file: String,
}

#[derive(Debug, Deserialize)]
struct Config {
    max_depth: usize,
    exclude_dirs: Vec<String>,
    project_markers: Vec<String>,
    naming: Naming,
    cache: CacheConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ProjectEntry {
    path: String,
    kind: String,
    marker: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct Cache {
    #[serde(default)]
    dirs: HashMap<String, u64>,
    #[serde(default)]
    projects: Vec<ProjectEntry>,
}

fn dir_mtime_secs(path: &Path) -> Option<u64> {
    let meta = fs::metadata(path).ok()?;
    let modified = meta.modified().ok()?;
    modified.duration_since(UNIX_EPOCH).ok().map(|d| d.as_secs())
}

fn classify(name: &str, repo_re: &Regex, domain_re: &Regex) -> Option<&'static str> {
    if domain_re.is_match(name) {
        Some("domain")
    } else if repo_re.is_match(name) {
        Some("repo")
    } else {
        None
    }
}

fn find_marker(dir: &Path, markers: &[String]) -> Option<String> {
    let entries = fs::read_dir(dir).ok()?;
    let names: Vec<String> = entries
        .filter_map(|e| e.ok())
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    markers.iter().find(|m| names.contains(m)).cloned()
}

fn scan(
    dir: &Path,
    depth: usize,
    config: &Config,
    repo_re: &Regex,
    domain_re: &Regex,
    old_cache: &Cache,
    new_dirs: &mut HashMap<String, u64>,
    projects: &mut Vec<ProjectEntry>,
) {
    if depth > config.max_depth {
        return;
    }

    let path_str = dir.to_string_lossy().to_string();
    let mtime = match dir_mtime_secs(dir) {
        Some(m) => m,
        None => return,
    };
    new_dirs.insert(path_str.clone(), mtime);

    let unchanged = old_cache.dirs.get(&path_str) == Some(&mtime);
    if unchanged {
        // Reuse previously discovered projects at or under this directory instead of rescanning.
        let prefix = format!("{path_str}{}", std::path::MAIN_SEPARATOR);
        let reused: Vec<ProjectEntry> = old_cache
            .projects
            .iter()
            .filter(|p| p.path == path_str || p.path.starts_with(&prefix))
            .cloned()
            .collect();
        if !reused.is_empty() || old_cache.projects.iter().all(|p| p.path != path_str) {
            projects.extend(reused);
            return;
        }
    }

    if let Some(marker) = find_marker(dir, &config.project_markers) {
        let name = dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let kind = classify(&name, repo_re, domain_re).unwrap_or("unknown");
        projects.push(ProjectEntry {
            path: path_str,
            kind: kind.to_string(),
            marker,
        });
        return; // Don't descend into a confirmed project root.
    }

    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.filter_map(|e| e.ok()) {
        let child = entry.path();
        if !child.is_dir() {
            continue;
        }
        let child_name = entry.file_name().to_string_lossy().to_string();
        if config.exclude_dirs.iter().any(|ex| ex == &child_name) {
            continue;
        }
        scan(
            &child, depth + 1, config, repo_re, domain_re, old_cache, new_dirs, projects,
        );
    }
}

/// リポジトリのGitHub remote URL(origin)を取得する。取得できなければNone。
fn git_remote_url(path: &Path) -> Option<String> {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(path)
        .args(["remote", "get-url", "origin"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() {
        None
    } else {
        // .git サフィックスを除去して見た目を揃える
        Some(s.trim_end_matches(".git").to_string())
    }
}

/// CLAUDE.md(無ければREADME.md)から一行程度の役割説明を抜き出す。
/// 「## ...役割...」「## ...とは...」見出し直後の本文、もしくは冒頭の説明文を拾う。
/// 見つからなければ None を返す(呼び出し側で「(説明未取得)」に差し替える)。
fn extract_description(path: &Path) -> Option<String> {
    let candidates = [path.join("CLAUDE.md"), path.join("README.md")];
    for file in candidates.iter() {
        let Ok(text) = fs::read_to_string(file) else {
            continue;
        };
        let lines: Vec<&str> = text.lines().collect();
        // 見出しに「役割」「とは」を含む行の直後の非空行を探す
        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim_start_matches('#').trim();
            if line.trim_start().starts_with('#') && (line.contains("役割") || line.contains("とは")) {
                for candidate in lines.iter().skip(i + 1) {
                    let c = candidate.trim();
                    if !c.is_empty() && !c.starts_with('#') && !c.starts_with('>') {
                        return Some(truncate_desc(c));
                    }
                }
            }
            let _ = trimmed;
        }
        // フォールバック: 最初の見出し以降で最初の本文段落
        let mut seen_heading = false;
        for line in lines.iter() {
            let t = line.trim();
            if t.starts_with('#') {
                seen_heading = true;
                continue;
            }
            if seen_heading && !t.is_empty() && !t.starts_with('>') && !t.starts_with('[') {
                return Some(truncate_desc(t));
            }
        }
    }
    None
}

fn truncate_desc(s: &str) -> String {
    let s = s.trim();
    let max_chars = 60;
    let truncated: String = s.chars().take(max_chars).collect();
    if s.chars().count() > max_chars {
        format!("{truncated}…")
    } else {
        truncated
    }
}

/// `open-raid-z/CLAUDE.md` の「関連プロジェクト」節を自動更新する。
/// 既存の手書き箇条書き(名前・説明・URL)はそのまま保持し、
/// まだ載っていないリポジトリだけをマーカーコメントで囲んだ自動生成ブロックへ追記する。
fn update_ecosystem_map(root: &Path, projects: &[ProjectEntry]) {
    let target = root.join("open-raid-z").join("CLAUDE.md");
    let Ok(original) = fs::read_to_string(&target) else {
        eprintln!("skip ecosystem map update: {} not found", target.display());
        return;
    };

    // 上書き前に必ずバックアップを取る
    let backup = root.join("open-raid-z").join("CLAUDE.md.bak");
    if fs::write(&backup, &original).is_err() {
        eprintln!("failed to write backup {}, aborting update", backup.display());
        return;
    }

    const SECTION_HEADING: &str = "## 関連プロジェクト";
    const AUTO_START: &str = "<!-- AUTO-GENERATED ECOSYSTEM MAP START (runo-scanner --update-ecosystem-map) -->";
    const AUTO_END: &str = "<!-- AUTO-GENERATED ECOSYSTEM MAP END -->";

    let Some(section_start) = original.find(SECTION_HEADING) else {
        eprintln!("section '{SECTION_HEADING}' not found in {}", target.display());
        return;
    };
    // 次の見出し("\n## " or "\n### ")の開始位置までがこの節
    let after_heading = section_start + SECTION_HEADING.len();
    let rest = &original[after_heading..];
    let section_end_rel = rest
        .find("\n### ")
        .or_else(|| rest.find("\n## "))
        .unwrap_or(rest.len());
    let section_body = &rest[..section_end_rel];

    // 重複追記防止のチェックは「手書き部分」のみを対象にする(前回の自動生成
    // ブロックは除外して毎回作り直す)。そうしないと2回目以降の実行で
    // 自動生成ブロックの中身が空のまま確定してしまうバグになる。
    let manual_body: String = if let Some(bs) = section_body.find(AUTO_START) {
        let be = section_body
            .find(AUTO_END)
            .map(|i| i + AUTO_END.len())
            .unwrap_or(section_body.len());
        format!("{}{}", &section_body[..bs], &section_body[be..])
    } else {
        section_body.to_string()
    };
    let already_covered = |url: &str, name: &str| -> bool {
        if manual_body.contains(url) {
            return true;
        }
        // 大文字小文字を無視した名前一致も許容(例: RGit -> RS-Git への改名は別名として一致しない)
        manual_body.contains(name)
    };

    // 自動生成ブロックがあれば中身を作り直す。無ければ節の末尾に新設する。
    let mut new_entries: Vec<String> = Vec::new();
    for p in projects {
        let path = Path::new(&p.path);
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        if name.is_empty() || name == "open-raid-z" {
            continue;
        }
        let Some(url) = git_remote_url(path) else {
            continue; // GitHub remote が無いローカル専用プロジェクトはマップに載せない
        };
        if already_covered(&url, &name) {
            continue;
        }
        let desc = extract_description(path).unwrap_or_else(|| "(説明未取得)".to_string());
        new_entries.push(format!("- **{name}**({desc}): {url}"));
    }
    new_entries.sort();
    new_entries.dedup();

    let auto_block = if new_entries.is_empty() {
        String::new()
    } else {
        format!(
            "\n{AUTO_START}\n{}\n{AUTO_END}\n",
            new_entries.join("\n")
        )
    };

    // 既存の自動生成ブロックがあれば置き換え、無ければ節の末尾に挿入
    let new_full = if section_body.contains(AUTO_START) {
        let block_start = section_body.find(AUTO_START).unwrap();
        let block_end = section_body
            .find(AUTO_END)
            .map(|i| i + AUTO_END.len())
            .unwrap_or(section_body.len());
        let mut new_body = String::new();
        new_body.push_str(&section_body[..block_start]);
        new_body.push_str(&auto_block.trim_start_matches('\n'));
        new_body.push_str(&section_body[block_end..]);
        format!(
            "{}{}{}",
            &original[..after_heading],
            new_body,
            &original[after_heading + section_end_rel..]
        )
    } else {
        format!(
            "{}{}{}{}",
            &original[..after_heading],
            section_body,
            auto_block,
            &original[after_heading + section_end_rel..]
        )
    };

    if new_entries.is_empty() && !section_body.contains(AUTO_START) {
        println!("ecosystem map: no new repositories to add");
        return;
    }

    // 実行の度に空行が積み重ならないよう、3行以上の連続空行は2行に正規化する
    let normalize_blank_lines = Regex::new(r"\n{3,}").unwrap();
    let new_full = normalize_blank_lines.replace_all(&new_full, "\n\n").to_string();

    match fs::write(&target, new_full) {
        Ok(_) => println!(
            "ecosystem map updated: {} ({} new entr{})",
            target.display(),
            new_entries.len(),
            if new_entries.len() == 1 { "y" } else { "ies" }
        ),
        Err(e) => eprintln!("failed to write {}: {e}", target.display()),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: runo-scanner <root-path> [config-path] [--update-ecosystem-map]");
        std::process::exit(1);
    }
    let update_ecosystem = args.iter().any(|a| a == "--update-ecosystem-map");
    let positional: Vec<&String> = args.iter().skip(1).filter(|a| !a.starts_with("--")).collect();
    let root = PathBuf::from(positional[0]);
    let config_path = positional
        .get(1)
        .map(|s| PathBuf::from(s))
        .unwrap_or_else(|| root.join("runo-scan.txt"));

    let config_text = fs::read_to_string(&config_path).unwrap_or_else(|e| {
        eprintln!("failed to read config {}: {e}", config_path.display());
        std::process::exit(1);
    });
    let config: Config = toml::from_str(&config_text).unwrap_or_else(|e| {
        eprintln!("failed to parse config {}: {e}", config_path.display());
        std::process::exit(1);
    });

    let repo_re = Regex::new(&config.naming.repo_name_pattern).expect("invalid repo_name_pattern");
    let domain_re =
        Regex::new(&config.naming.domain_name_pattern).expect("invalid domain_name_pattern");

    let cache_path = root.join(&config.cache.index_file);
    let old_cache: Cache = fs::read_to_string(&cache_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();

    let mut new_dirs = HashMap::new();
    let mut projects = Vec::new();
    scan(
        &root, 0, &config, &repo_re, &domain_re, &old_cache, &mut new_dirs, &mut projects,
    );

    let new_cache = Cache {
        dirs: new_dirs,
        projects: projects.clone(),
    };
    if let Ok(json) = serde_json::to_string_pretty(&new_cache) {
        let _ = fs::write(&cache_path, json);
    }

    println!("{}", serde_json::to_string_pretty(&projects).unwrap());

    if update_ecosystem {
        update_ecosystem_map(&root, &projects);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn res() -> (Regex, Regex) {
        (
            Regex::new(r"^[A-Za-z0-9]+(-[A-Za-z0-9]+)*$").unwrap(),
            Regex::new(r"^[a-z0-9-]+\.[a-z]{2,}$").unwrap(),
        )
    }

    #[test]
    fn classify_distinguishes_repo_and_domain() {
        let (repo, domain) = res();
        assert_eq!(classify("open-web-server", &repo, &domain), Some("repo"));
        assert_eq!(classify("RS-Git", &repo, &domain), Some("repo"));
        assert_eq!(classify("aon.tokyo", &repo, &domain), Some("domain"));
        assert_eq!(classify("not a name!", &repo, &domain), None);
    }

    #[test]
    fn truncate_desc_keeps_short_and_cuts_long() {
        assert_eq!(truncate_desc("  short  "), "short");
        let long = "あ".repeat(100);
        let out = truncate_desc(&long);
        assert_eq!(out.chars().count(), 61);
        assert!(out.ends_with('…'));
    }

    #[test]
    fn find_marker_returns_first_matching_marker() {
        let dir = std::env::temp_dir().join(format!("runo-scanner-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("package.json"), "{}").unwrap();
        let markers = vec!["Cargo.toml".to_string(), "package.json".to_string()];
        assert_eq!(find_marker(&dir, &markers), Some("package.json".to_string()));
        assert_eq!(find_marker(&dir, &["nope".to_string()]), None);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn example_config_parses() {
        let text = include_str!("../runo-scan.example.txt");
        let cfg: Config = toml::from_str(text).expect("example config must parse");
        assert!(cfg.max_depth > 0);
        assert!(cfg.project_markers.contains(&"Cargo.toml".to_string()));
    }
}
