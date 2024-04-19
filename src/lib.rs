#![deny(clippy::all)]

pub mod changelog;
pub mod error;
pub mod npm;
use napi_derive::napi;

use std::{
  fs::{self, create_dir, File},
  io::Write,
  path::{Path, PathBuf},
  string::String,
};

pub use crate::error::{Error, ErrorKind, Result};
use crate::{changelog::Changelogs, npm::Npm};

#[derive(Debug)]
struct ReadFileError();

fn create_md_file(package_path: String, content: String) {
  let mut buffer = File::create(package_path).unwrap();
  buffer.write_all(content.as_bytes()).unwrap();
  buffer.flush().unwrap();
  buffer.sync_all().unwrap();
}

#[napi]
pub fn gen_changelogs(repo: String, changelog_path: Option<String>) {
  let mut repo_changelog_path = PathBuf::new();
  let changelog_path = match changelog_path {
    Some(p) => p,
    None => ".changelogs".to_string(),
  };

  repo_changelog_path.push(repo.clone());
  repo_changelog_path.push(changelog_path);

  let dir_path = repo_changelog_path.display().to_string();
  if Path::new(&dir_path).exists() {
    fs::remove_dir_all(repo_changelog_path.display().to_string())
      .expect(format!("删除文件失败 {dir_path}", dir_path = dir_path).as_str());
  }
  create_dir(dir_path).expect("创建 changelog 文件夹失败");

  // 只写入 latest
  let mut md_path = repo_changelog_path.clone();

  md_path.push("components.md");

  let mut md_str_list: Vec<String> = vec![];

  let md_file_content_list = Changelogs::new(repo).get_change_log_list();
  for md_file_content in md_file_content_list {
    println!("-> 正在生成 {} 的 changelog", md_file_content.package);
    md_str_list.push(md_file_content.content);
  }

  create_md_file(md_path.display().to_string(), md_str_list.join(""));

  println!("{:?}", "🆗 生成完成。");
}

#[napi]
pub fn gen_all_changelogs(repo: String, changelog_path: Option<String>) {
  let mut repo_changelog_path = PathBuf::new();
  let changelog_path = match changelog_path {
    Some(p) => p,
    None => ".changelogs".to_string(),
  };
  repo_changelog_path.push(repo.clone());
  repo_changelog_path.push(changelog_path);

  let dir_path = repo_changelog_path.display().to_string();
  if Path::new(&dir_path).exists() {
    fs::remove_dir_all(repo_changelog_path.display().to_string())
      .expect(format!("删除文件失败 {dir_path}", dir_path = dir_path).as_str());
  }
  create_dir(dir_path).expect("创建 changelog 文件夹失败");

  // 只写入 latest
  let md_file_content_list = Changelogs::new(repo).get_all_change_log_list();
  let mut md_path = repo_changelog_path.clone();
  md_path.push("components.md");
  let mut md_str_list: Vec<String> = vec![];

  for md_file_content in md_file_content_list {
    md_str_list.push(md_file_content.content.to_string());
    println!("-> 正在生成 {} 的 changelog", md_file_content.package);
  }

  create_md_file(md_path.display().to_string(), md_str_list.join(""));

  println!("{:?}", "🆗 生成完成。");
}

#[napi]
pub fn check_publish(repo: String) {
  Npm::new(repo).check();
}

#[cfg(test)]
mod tests {
  use std::{env, path::Path};

  use crate::{check_publish, gen_all_changelogs, gen_changelogs};
  #[test]
  fn it_gen_changelogs() {
    let token = &env::var("GITHUB_TOKEN").expect("未找到 GITHUB_TOKEN");
    println!("{:?}", token);
    if Path::new("/Users/shuaichen/Documents/github/pro-components").exists() {
      gen_changelogs(
        "/Users/shuaichen/Documents/github/pro-components".to_string(),
        Some(".changhelog2".to_string()),
      );
    }
    assert_eq!(2 + 2, 4);
  }

  #[test]
  fn it_gen_all_changelogs() {
    if Path::new("/Users/shuaichen/Documents/github/pro-components").exists() {
      gen_all_changelogs(
        "/Users/shuaichen/Documents/github/pro-components".to_string(),
        None,
      );
    }
    assert_eq!(2 + 2, 4);
  }

  #[test]
  fn it_check_publish() {
    if Path::new("/Users/shuaichen/Documents/github/pro-components").exists() {
      check_publish("/Users/shuaichen/Documents/github/pro-components".to_string());
    }
    if Path::new("C:/github/pro-components").exists() {
      check_publish("C:/github/pro-components".to_string());
    }
    assert_eq!(2 + 2, 4);
  }
}
