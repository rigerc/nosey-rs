//! Initial native dependency providers. Unknown providers are never treated as negative matches.
use serde_json::Value;
use crate::{jsonc, mcp, yaml_providers};
use std::collections::BTreeSet;
pub(crate) fn supported(e:&str)->bool{matches!(e,"npm"|"composer"|"deno"|"cargo"|"python"|"go"|"ruby"|"hex"|"docker"|"githubAction")}
fn base(path:&str)->&str{path.rsplit('/').next().unwrap_or(path)}
pub(crate) fn applies(e:&str,path:&str)->bool{let b=base(path);match e{"mcp"=>mcp::applies(path),"githubAction"=>path.contains(".github/workflows/"),"npm"=>b=="package.json","composer"=>b=="composer.json","deno"=>matches!(b,"deno.json"|"deno.jsonc"),"cargo"=>b=="Cargo.toml","python"=>b=="pyproject.toml"||b=="Pipfile"||b.starts_with("requirements"),"go"=>b=="go.mod","ruby"=>matches!(b,"Gemfile"|"Gemfile.lock"),"hex"=>matches!(b,"mix.exs"|"mix.lock"),"docker"=>b.starts_with("Dockerfile")||matches!(b,"compose.yaml"|"compose.yml"|"docker-compose.yml"|"docker-compose.yaml"),_=>false}}
fn python_name(s:&str)->Option<String>{s.split(['[','<','>','=','!','~',';',' ','@']).find(|x|!x.is_empty()).map(|x|x.to_ascii_lowercase().replace('_',"-"))}
fn toml_names(v:&toml::Value,e:&str,out:&mut BTreeSet<String>){if let Some(t)=v.as_table(){for (k,v) in t{if let Some(table)=v.as_table(){if matches!(k.as_str(),"dependencies"|"dev-dependencies"|"build-dependencies"|"packages"|"dev-packages"){for (name,spec) in table{let name=if e=="cargo"{spec.get("package").and_then(toml::Value::as_str).unwrap_or(name)}else{name};out.insert(name.to_owned());}}if e=="python"&&matches!(k.as_str(),"optional-dependencies"|"dependency-groups"){for group in table.values(){if let Some(list)=group.as_array(){out.extend(list.iter().filter_map(toml::Value::as_str).filter_map(python_name));}}}toml_names(v,e,out);}else if e=="python"&&matches!(k.as_str(),"dependencies"|"optional-dependencies"){if let Some(list)=v.as_array(){out.extend(list.iter().filter_map(toml::Value::as_str).filter_map(python_name));}}}}}
pub(crate) fn names(e:&str,path:&str,bytes:&[u8])->Result<BTreeSet<String>,&'static str>{let text=std::str::from_utf8(bytes).map_err(|_|"invalid-manifest-text")?;let mut out=BTreeSet::new();match e{
    "npm"|"composer"|"deno"=>{let normalized;let input=if base(path)=="deno.jsonc"{normalized=jsonc::strip(text)?;normalized.as_slice()}else{bytes};let doc:Value=serde_json::from_slice(input).map_err(|_|"malformed-json-manifest")?;if !doc.is_object(){return Err("malformed-json-manifest");}for key in ["dependencies","devDependencies","peerDependencies","optionalDependencies","require","require-dev","imports"]{if let Some(v)=doc.get(key){let m=v.as_object().ok_or("invalid-dependency-table")?;for (name,value) in m{if e=="deno"{out.insert(value.as_str().ok_or("invalid-dependency-value")?.to_owned());}else{out.insert(name.clone());}}}}},
    "cargo"|"python" if base(path).ends_with(".toml")||base(path)=="Pipfile"=>{let doc:toml::Value=toml::from_str(text).map_err(|_|"malformed-toml-manifest")?;toml_names(&doc,e,&mut out);},
    "python"=>{for line in text.lines(){let s=line.trim();if !s.is_empty()&&!s.starts_with('#')&&!s.starts_with('-'){if let Some(n)=python_name(s){out.insert(n);}}}},
    "go"|"ruby"=>{let r=regex::Regex::new(r#"(?m)^\s*(?:require\s+|gem\s+)?["']?([A-Za-z0-9@_./+-]+)["']?\s*(?:[=,: ]|$)"#).map_err(|_|"provider-regex")?;for m in r.captures_iter(text){if let Some(n)=m.get(1){let n=n.as_str();if e!="go"||(n.contains('.')&&n.contains('/')){out.insert(n.to_owned());}}}},
    "hex"=>{let r=regex::Regex::new(r"\{\s*:([A-Za-z0-9_]+)\s*,").map_err(|_|"provider-regex")?;for m in r.captures_iter(text){if let Some(n)=m.get(1){out.insert(n.as_str().to_owned());}}},
    "docker"=>{if !base(path).starts_with("Dockerfile"){return yaml_providers::compose(text);}for line in text.lines(){let f:Vec<_>=line.split_whitespace().collect();if f.first().is_some_and(|s|s.eq_ignore_ascii_case("FROM")){let i=if f.get(1).is_some_and(|s|s.starts_with("--")){2}else{1};let n=f.get(i).ok_or("malformed-dockerfile")?;if n.contains('$'){return Err("dynamic-docker-image");}let n=n.split('@').next().unwrap_or(n);let n=if let Some(i)=n.rfind(':'){if n.rfind('/').is_none_or(|j|i>j){&n[..i]}else{n}}else{n};out.insert(n.to_owned());}}},
    "githubAction"=>return yaml_providers::actions(text),
    "mcp"=>return mcp::names(path,bytes),
    _=>return Err("unsupported-provider")};Ok(out)}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn providers_dispatch_and_match_existing_rule_grammar() -> Result<(), &'static str> {
        assert!(supported("githubAction"));
        assert!(applies("githubAction", "nested/.github/workflows/ci.yml"));
        assert!(applies("mcp", ".codex/config.toml"));
        assert_eq!(names("deno", "deno.jsonc", br#"{"imports":{"x":"npm:sample@1",},//comment
        }"#)?, BTreeSet::from(["npm:sample@1".into()]));
        assert_eq!(names("docker", "compose.yaml", b"services:\n  app:\n    image: example/app:1\n")?, BTreeSet::from(["example/app".into()]));
        assert_eq!(names("githubAction", ".github/workflows/ci.yml", b"jobs:\n  build:\n    steps:\n      - uses: actions/checkout@v4\n")?, BTreeSet::from(["actions/checkout".into()]));
        Ok(())
    }
}
