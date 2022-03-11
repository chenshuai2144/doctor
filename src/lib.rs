#![deny(clippy::all)]

use context::Context;
pub mod changelog;
pub mod context;
pub mod diagnostic;
pub mod error;
pub mod handler;
pub mod npm;
pub mod rules;

use diagnostic::display_diagnostics;
use napi_derive::napi;
use rules::get_all_rules_raw;

use std::{
  fs::{self, create_dir, File},
  io::Write,
  path::{Path, PathBuf},
  result,
  string::String,
};

pub use crate::error::{Error, ErrorKind, Result};
use crate::{changelog::Changelogs, npm::Npm};

#[derive(Debug)]
struct ReadFileError(String);

/**
 * 把文件内容转化为语法树
 */
fn parse_program(
  file_name: &str,
  syntax: deno_ast::swc::parser::Syntax,
  source_code: String,
) -> result::Result<deno_ast::ParsedSource, deno_ast::Diagnostic> {
  deno_ast::parse_program(deno_ast::ParseParams {
    specifier: file_name.to_string(),
    media_type: deno_ast::MediaType::Unknown,
    source: deno_ast::SourceTextInfo::from_string(source_code),
    capture_tokens: true,
    maybe_syntax: Some(syntax),
    scope_analysis: true,
  })
}

#[napi]
pub fn check_routers(path_str: String) -> bool {
  // 有没有通过lint
  let mut pass = false;
  // 读取文件内容
  let content = std::fs::read_to_string(&path_str)
    .map_err(|err| ReadFileError(format!("读取文件异常： `{}`: {}", path_str, err)))
    .unwrap();

  // 定义一下是一个 ts ast 的格式
  let syntax = deno_ast::get_syntax(deno_ast::MediaType::TypeScript);
  // 转化为语法树
  let ast = parse_program(&path_str, syntax, content).unwrap();

  ast.with_view(|program| {
    // 生成一个context，用于存储错误信息并且被各个规则消费
    let mut context = Context::new(
      path_str.to_string().clone(),
      deno_ast::MediaType::TypeScript,
      ast.source(),
      program,
    );

    let rules = get_all_rules_raw();
    for rule in rules {
      rule.lint_program_with_ast_view(&mut context, program);
    }

    if context.diagnostics().is_empty() {
      println!("👍 没有发现任何问题，非常好!");
      pass = true;
    }

    display_diagnostics(&context.diagnostics(), ast.source());
  });

  pass = false;

  pass
}

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
  let md_file_content_list = Changelogs::new(repo).get_change_log_list();
  for md_file_content in md_file_content_list {
    let mut md_path = repo_changelog_path.clone();
    md_path.push(format!("{package}.md", package = md_file_content.package));
    println!("-> 正在生成 {} 的 changelog", md_file_content.package);
    create_md_file(md_path.display().to_string(), md_file_content.content);
  }
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
  for md_file_content in md_file_content_list {
    let mut md_path = repo_changelog_path.clone();
    md_path.push(format!("{package}.md", package = md_file_content.package));
    println!("-> 正在生成 {} 的 changelog", md_file_content.package);
    create_md_file(md_path.display().to_string(), md_file_content.content);
  }
  println!("{:?}", "🆗 生成完成。");
}

#[napi]
pub fn check_publish(repo: String) {
  Npm::new(repo).check();
}

#[cfg(test)]
mod tests {
  use std::path::Path;

  use crate::{check_publish, check_routers, gen_all_changelogs, gen_changelogs};

  #[test]
  fn it_gen_changelogs() {
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
    assert_eq!(2 + 2, 4);
  }
  #[test]
  fn it_check_routers() {
    if Path::new("/Users/shuaichen/Documents/github/pro-components").exists() {
      check_routers(
        "/Users/shuaichen/Documents/github/ant-design-pro/config/routes.ts".to_string(),
      );
    }
    assert_eq!(2 + 2, 4);
  }
}
