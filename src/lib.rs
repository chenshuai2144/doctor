#![deny(clippy::all)]

use context::Context;
pub mod context;
pub mod diagnostic;
pub mod handler;
pub mod rules;

use diagnostic::display_diagnostics;
use napi_derive::napi;
use rules::get_all_rules_raw;
use std::string::String;

#[derive(Debug)]
struct ReadFileError(String);

/**
 * 把文件内容转化为语法树
 */
fn parse_program(
  file_name: &str,
  syntax: deno_ast::swc::parser::Syntax,
  source_code: String,
) -> Result<deno_ast::ParsedSource, deno_ast::Diagnostic> {
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
pub fn check(path_str: String) -> bool {
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
