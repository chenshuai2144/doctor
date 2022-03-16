use git2::Repository;
use reqwest::Client;
use semver::Version;
use serde::Deserialize;
use serde_json;
use std::env::consts::OS;
use std::path::Path;
use std::{collections::HashMap, env, fs, io, process::Command};

use crate::changelog::git::get_version;

#[cfg(windows)]
pub const NPM: &'static str = "npm.cmd";

#[cfg(not(windows))]
pub const NPM: &'static str = "npm";

#[derive(Deserialize)]
pub struct NpmPackageInfo {
  name: String,
  version: String,
}

pub struct Npm {
  client: Client,
  path: String,
  package_list: Vec<NpmPackageInfo>,
}

impl Npm {
  /* 如果有发布失败的包，那么就不执行 npm dist-tag add latest */
  pub fn check(&self) {
    let map = self.check_package_list_publish_success();

    let all_published = map.iter().any(|(package, published)| -> bool {
      if published.to_owned().to_owned() {
        return true;
      }
      println!("😟 {} 发布失败！", package);
      false
    });

    if all_published {
      println!("🆗 全部发布成功");
      let npm_path = self.get_path();

      for package_version in &self.package_list {
        println!(
          "📕 即将执行 npm dist-tag add {} latest",
          format!(
            "{name}@{version}",
            name = package_version.name,
            version = package_version.version
          )
        );
        println!("请输入opt,如果没有请留空：");

        let mut input = String::new();

        io::stdin().read_line(&mut input).expect("读取失败");

        let output = Command::new(NPM)
          .env("NPM_CONFIG_OTP", input.trim())
          .current_dir(npm_path.clone())
          .arg("dist-tag")
          .arg("add")
          .arg(format!(
            "{name}@{version}",
            name = package_version.name,
            version = package_version.version
          ))
          .arg("latest")
          .spawn()
          .expect("执行异常，提示")
          .wait_with_output()
          .unwrap();

        let output_string = String::from_utf8_lossy(&output.stderr);

        if !output_string.is_empty() {
          println!(
            "{}",
            output_string.split("\n").collect::<Vec<&str>>().join("\n")
          );
        }
        let output_string = String::from_utf8_lossy(&output.stdout).to_string();

        if !output_string.is_empty() {
          println!(
            "{}",
            output_string.split("\n").collect::<Vec<&str>>().join("\n")
          );
        }
      }
    } else {
      println!("😟 发布失败了，等待 npm 回复再转化为正式版本。");
    }
  }
  /* 判断这个包是不是发布成功了 */
  pub fn check_package_list_publish_success(&self) -> HashMap<String, bool> {
    let mut map: HashMap<String, bool> = HashMap::new();
    for package_info in &self.package_list {
      let is_publish =
        self.check_publish_success(package_info.name.as_str(), package_info.version.as_str());
      map.insert(package_info.name.clone(), is_publish);
    }
    map
  }

  /**
   * 判断这个版本是不是发布成功了
   */
  pub fn check_publish_success(&self, name: &str, version: &str) -> bool {
    let endpoint = format!(
      "https://registry.npmjs.org/{name}/{version}",
      name = name,
      version = version
    );

    self
      .client
      .get(&endpoint)
      .send()
      .unwrap()
      .json::<NpmPackageInfo>()
      .is_ok()
  }

  /**
   * 获取  latest 的最后一个版本
   */
  pub fn get_package_latest_version(&self, name: &str) -> String {
    let endpoint = format!("https://registry.npmjs.org/{name}/latest", name = name,);

    self
      .client
      .get(&endpoint)
      .send()
      .unwrap()
      .json::<NpmPackageInfo>()
      .unwrap()
      .version
  }

  /* 获取 nodejs 的安装路径 */
  fn get_path(&self) -> String {
    if OS == "windows" {
      return env::var("path")
        .expect("获取 path 失败")
        .split(";")
        .find(|path| {
          if path.contains("nodejs") {
            return true;
          }
          false
        })
        .unwrap()
        .to_string();
    }
    self.path.clone()
  }
  /* 获取 package.json 中的 version 字段 */
  pub fn get_pre_package_version(&self) -> Vec<String> {
    let repo = Repository::open(&self.path).unwrap();
    let mut tag_list = repo
      .tag_names(None)
      .unwrap()
      .iter()
      .filter_map(|tag| {
        Version::parse(&get_version(tag.unwrap()).to_owned().version)
          .ok()
          .map(|version| (tag.unwrap().to_string(), version))
      })
      .collect::<Vec<_>>();

    tag_list.sort_by(|(_, a), (_, b)| b.cmp(a));

    let sort_tags = tag_list
      .into_iter()
      .map(|(tag, _)| -> String { tag })
      .collect::<Vec<String>>();

    let pre_package_version = self
      .package_list
      .iter()
      .map(|package| -> String {
        let package_name = package.name.as_str();
        let tag = sort_tags
          .clone()
          .into_iter()
          .filter(|tag| tag.contains(package_name))
          .collect::<Vec<_>>()
          .get(1)
          .unwrap()
          .clone();
        tag
      })
      .collect();

    pre_package_version
  }
  pub fn new(path: String) -> Npm {
    let client = Client::new();
    let packages_path = format!("{path}/packages/", path = path);
    let package_list: Vec<NpmPackageInfo> = fs::read_dir(&packages_path)
      .unwrap()
      .filter(|entry| {
        let entry = entry.as_ref().unwrap();
        let path = entry.path();
        let path = path.to_str().unwrap();
        Path::new(path).is_dir()
      })
      .map(|entry| {
        let entry = entry.unwrap();
        let path = entry.path();
        let path = path.to_str().unwrap();

        let data = fs::read_to_string(format!("{path}/package.json", path = path))
          .expect(format!("{path}/package.json", path = path).as_str());

        let package_info: NpmPackageInfo =
          serde_json::from_str(&data).expect("格式化  package.json失败 ");

        return package_info;
      })
      .collect();

    println!("🔍 发现了{} 个 包 ->", &package_list.len());
    println!("-------------------");
    for package in &package_list {
      println!("📦 {}", package.name)
    }

    println!("🔚🔚🔚🔚🔚🔚🔚🔚🔚🔚🔚");

    Npm {
      path,
      client,
      package_list,
    }
  }
}
