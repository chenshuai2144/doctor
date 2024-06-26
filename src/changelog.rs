﻿pub mod git;
use git2::Repository;
use regex::Regex;
use reqwest::Client;
use serde::Deserialize;
use std::{collections::HashMap, env, ops::Index};

use self::git::{full_commits, latest_commits, Commit};

pub struct Changelogs {
  repo: Repository,
  author_github_map: HashMap<String, String>,
  client: Client,
  github_html_url: String,
  repo_name: String,
}

#[derive(Debug)]
pub struct MARKDOWN {
  pub package: String,
  pub content: String,
}

#[derive(Deserialize)]
struct GithubUser {
  login: String,
}

#[derive(Deserialize)]
struct GithubPull {
  user: GithubUser,
}

#[derive(Deserialize)]
struct GithubRepo {
  html_url: String,
}

impl Changelogs {
  pub fn gen_change_log_by_commit_list(
    &mut self,
    commit_list: Vec<Commit>,
    package: &str,
  ) -> crate::Result<Vec<String>> {
    let mut changelog_list: Vec<String> = vec![];

    let mut commit_hash_map: HashMap<String, bool> = HashMap::new();

    for commit in commit_list {
      let message = commit.message().split("\n").nth(0).expect("信息转化失败");
      let hash = commit.hash().to_string();

      let re = Regex::new(r"[fix|feat]\(([0-9a-zA-Z_]*)\)").expect("正则表达式转化失败");

      let mut need_insert_message = false;

      if re.is_match(message) {
        if re
          .captures(message)
          .expect("正则表达式转化失败")
          .index(1)
          .to_lowercase()
          .eq(package)
        {
          need_insert_message = true
        }
      }

      if need_insert_message && !commit_hash_map.get(&hash).is_some() {
        let md_message = self.get_md_message(&commit);
        changelog_list.insert(changelog_list.len(), md_message.clone());

        commit_hash_map.insert(hash, true);
      }
    }

    Ok(changelog_list)
  }
  pub fn gen_change_log_to_md(&mut self, change_logs: Vec<String>) -> String {
    let mut md_file_content: String = "".to_owned();

    for changelog in change_logs {
      // 格式化成这个样子
      //  * feat(layout): mix support headerContent render [@chenshuai2144](https://github.com/chenshuai2144)
      md_file_content.push_str(&("* ".to_owned() + &changelog + "\n"));
    }

    md_file_content
  }

  /**
   * 获取所有的changelog
   * 会遍历所有的标签
   */
  pub fn get_all_change_log_list(&mut self) -> Vec<MARKDOWN> {
    let mut md_packages: Vec<MARKDOWN> = vec![];
    let package_list = [
      "components",
      "utils",
      "layout",
      "form",
      "list",
      "table",
      "field",
      "card",
      "descriptions",
    ];
    for package in package_list {
      let mut package_md: Vec<String> = vec![];
      let commit_and_tag_list =
        full_commits(&self.repo, &("@ant-design/pro-".to_owned() + package))
          .expect("获取commit失败");

      for commit_and_tag in commit_and_tag_list {
        let change_logs = self
          .gen_change_log_by_commit_list(commit_and_tag.commit_list, package)
          .expect("生成changelog 失败，请重试");

        let md_file_content = self.gen_change_log_to_md(change_logs);

        let tag = commit_and_tag.tag;
        if package == "components" {
          package_md.push(("\n## ".to_owned() + tag.name.as_str() + "\n\n").to_string());
          package_md.push(format!("`{date_time}`\n\n", date_time = tag.date_time).to_string());
        }

        package_md.insert(package_md.len(), md_file_content);
      }

      md_packages.insert(
        md_packages.len(),
        MARKDOWN {
          package: package.to_owned(),
          content: package_md.join(""),
        },
      )
    }
    md_packages
  }

  // 获取所有包的change log，会循环一下
  pub fn get_change_log_list(&mut self) -> Vec<MARKDOWN> {
    let mut md_packages: Vec<MARKDOWN> = vec![];
    let package_list = [
      "components",
      "utils",
      "layout",
      "form",
      "list",
      "table",
      "field",
      "card",
      "descriptions",
    ];

    for package in package_list {
      let (tag, commit_list) =
        latest_commits(&self.repo, &("@ant-design/pro-".to_owned() + package))
          .expect("获取包名失败");

      let change_logs = self
        .gen_change_log_by_commit_list(commit_list, package)
        .expect("生成changelog 失败，请重试");

      let mut md_file_content: String = "".to_owned();
      if package == "components" {
        md_file_content.push_str(&("## ".to_owned() + tag.name.as_str() + "\n\n"));
        md_file_content.push_str(format!("`{date_time}`\n\n", date_time = tag.date_time).as_str());
      }

      md_file_content.push_str(self.gen_change_log_to_md(change_logs).as_str());

      md_packages.insert(
        md_packages.len(),
        MARKDOWN {
          package: package.to_owned(),
          content: md_file_content,
        },
      );
    }

    md_packages
  }

  pub fn get_md_message(&mut self, commit: &Commit) -> String {
    let message = commit
      .message()
      .split("\n")
      .nth(0)
      .expect(" 信息不存在")
      .trim();

    let author = commit.author().as_ref().expect("author 不存在");
    let md_hash = commit.hash().trim();
    let short_md_hash = &md_hash[0..7];

    let re = Regex::new(r"\(#[0-9]*\)").unwrap();

    if re.is_match(message) {
      let pr_id = re
        .captures(message)
        .unwrap()
        .index(0)
        .replace("(", "")
        .replace(")", "");
      let github_user_id = self.get_pr_user_name(&pr_id, author);
      let pr_url = format!(
        "{github_url}/pull/{pr_id}",
        github_url = self.github_html_url,
        pr_id = pr_id
      );

      let md_message = format!(
        "{message}. [{pr_id}]({pr_url}) [@{github_user_id}](https://github.com/{github_user_id})",
        pr_id = pr_id,
        message = message,
        pr_url = pr_url,
        github_user_id = github_user_id,
      );

      return md_message;
    }

    let commit_or_pr_url = format!(
      "{github_url}/commit/{short_md_hash}",
      github_url = self.github_html_url,
      short_md_hash = short_md_hash
    );

    let md_message = format!(
      "{message}. [{short_md_hash}]({commit_or_pr_url})",
      short_md_hash = short_md_hash,
      message = message,
      commit_or_pr_url = commit_or_pr_url,
    );

    md_message
  }

  /**
   * 通过pr的name 获取真实姓名，不让name 和 id 对不上
   */
  pub fn get_pr_user_name(&mut self, pr_number: &str, author: &str) -> String {
    if self.author_github_map.get(author).is_none() {
      let pr_url = format!(
        "{github_url}{repo_name}/pulls/{pr_number}",
        github_url = " https://api.github.com/repos/",
        pr_number = pr_number.replace("#", "").trim(),
        repo_name = self.repo_name,
      );

      let body = self
        .client
        .get(&pr_url)
        .header(
          "Authorization",
          "token ".to_owned() + &env::var("GITHUB_TOKEN").expect("GITHUB_TOKEN 未找到"),
        )
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .unwrap()
        .json::<GithubPull>();

      if body.is_ok() {
        let pr = body.unwrap();
        self
          .author_github_map
          .insert(author.to_owned(), pr.user.login.to_owned());
      }
    }

    // 返回 map 里面对于 name 的映射
    let author_for_map = self.author_github_map.get(author);
    if author_for_map.is_some() {
      return author_for_map.unwrap().to_string();
    }
    author.to_string()
  }
  /**
   * 初始化，需要添加项目的地址
   */
  pub fn new(repo: String) -> Changelogs {
    let author_github_map = HashMap::new();
    let client = Client::new();
    let repo = Repository::open(repo).unwrap();

    //  仓库的 http 地址，用于生成 commit 的链接
    let repo_name = repo
      .find_remote("origin")
      .unwrap()
      .url()
      .unwrap()
      // git@github.com:ant-design/pro-components.git
      // -> ant-design/pro-components.git
      .split(":")
      .nth(1)
      .unwrap()
      .split(".")
      // ant-design/pro-components.git -> ant-design/pro-components
      .nth(0)
      .unwrap()
      .to_owned();

    let url = format!(
      "https://api.github.com/repos/{repo_name}",
      repo_name = repo_name
    );

    let body: GithubRepo = client
      .get(&url)
      .header("Accept", "application/vnd.github.v3+json")
      .send()
      .unwrap()
      .json()
      .expect("json 转化失败，请检查是网络错误，或者 GITHUB_TOKEN 是否失效！");

    let html_url = body.html_url;

    Changelogs {
      repo,
      client: client,
      author_github_map: author_github_map,
      github_html_url: html_url,
      repo_name: repo_name,
    }
  }
}
