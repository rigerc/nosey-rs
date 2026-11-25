//! Compiled rule predicates; unsupported mechanisms fail closed.
use globset::{GlobBuilder,GlobMatcher};
use nosey_registry::RuleKind;
use regex::{Regex,RegexBuilder};
use crate::providers;
fn regex(pattern:&str,insensitive:bool)->Result<Regex,&'static str>{if pattern.is_empty()||pattern.len()>4096{return Err("invalid-regex");}RegexBuilder::new(pattern).case_insensitive(insensitive).size_limit(1024*1024).dfa_size_limit(1024*1024).build().map_err(|_|"unsupported-regex")}
fn glob(pattern:&str)->Result<GlobMatcher,&'static str>{if pattern.is_empty()||pattern.len()>4096{return Err("invalid-glob");}GlobBuilder::new(pattern).literal_separator(true).build().map(|g|g.compile_matcher()).map_err(|_|"invalid-glob")}
pub(crate) enum Name {Literal(String),Pattern(Regex)}
impl Name {fn new(lit:&str,pat:&str)->Result<Self,&'static str>{if !pat.is_empty(){Ok(Self::Pattern(regex(pat,false)?))}else if !lit.is_empty(){Ok(Self::Literal(lit.into()))}else{Err("empty-name")}}fn matches(&self,value:&str)->bool{match self{Self::Literal(s)=>s==value,Self::Pattern(r)=>r.is_match(value)}}}
pub(crate) enum Matcher {FileGlob(GlobMatcher),FileRegex(Regex,bool),Extension(String),Content(GlobMatcher,Name),Dotenv(Option<GlobMatcher>,Name),Dependency(String,Name)}
impl Matcher {
    pub(crate) fn new(rule:&RuleKind,mcp:bool)->Result<Self,&'static str>{match rule{
        RuleKind::File{glob:g,path_regex,path_scope}=>if g.is_empty(){Ok(Self::FileRegex(regex(path_regex,false)?,path_scope=="basename"))}else{Ok(Self::FileGlob(glob(g)?))},
        RuleKind::Extension{extension} if !extension.is_empty()=>Ok(Self::Extension(extension.clone())),
        RuleKind::Content{glob:g,contains,regex:r,regex_flags}=>{let name=if !contains.is_empty(){Name::Literal(contains.clone())}else{if !regex_flags.is_empty()&&regex_flags!="i"{return Err("unsupported-regex-flags");}Name::Pattern(regex(r,regex_flags=="i")?)};Ok(Self::Content(glob(g)?,name))},
        RuleKind::Dotenv{glob:g,key,key_regex}=>Ok(Self::Dotenv(if g.is_empty(){None}else{Some(glob(g)?)},Name::new(key,key_regex)?)),
        RuleKind::Dependency{ecosystem,name,name_regex}=>{if !providers::supported(ecosystem){return Err("unsupported-provider");}Ok(Self::Dependency(ecosystem.clone(),Name::new(name,name_regex)?))},
        RuleKind::McpPackage{..} if !mcp=>Err("mcp-disabled"),RuleKind::McpPackage{name,name_regex}=>Ok(Self::Dependency("mcp".into(),Name::new(name,name_regex)?)),
        RuleKind::Detector{..}=>Err("external-detector-unavailable"),_=>Err("unsupported-rule")}}
    fn base(path:&str)->&str{path.rsplit('/').next().unwrap_or(path)}
    fn dotenv(g:Option<&GlobMatcher>,path:&str)->bool{g.map_or_else(||{let b=Self::base(path);b.starts_with(".env")&&!b.contains("example")&&!b.contains("sample")},|x|x.is_match(path))}
    pub(crate) fn needs_bytes(&self,path:&str)->bool{match self{Self::Content(g,_)=>g.is_match(path),Self::Dotenv(g,_)=>Self::dotenv(g.as_ref(),path),Self::Dependency(e,_)=>providers::applies(e,path),_=>false}}
    pub(crate) fn matches(&self,path:&str,is_dir:bool,bytes:Option<&[u8]>)->Result<Vec<Option<String>>,&'static str>{
        let hit=|yes|if yes{vec![None]}else{Vec::new()};
        match self{Self::FileGlob(g)=>Ok(hit(g.is_match(path))),Self::FileRegex(r,b)=>Ok(hit(r.is_match(if *b{Self::base(path)}else{path}))),_ if is_dir=>Ok(Vec::new()),Self::Extension(s)=>Ok(hit(path.ends_with(s.as_str()))),
            Self::Content(g,n) if g.is_match(path)=>{let text=std::str::from_utf8(bytes.ok_or("missing-file-data")?).map_err(|_|"invalid-text")?;Ok(hit(match n{Name::Literal(s)=>text.contains(s.as_str()),Name::Pattern(r)=>r.is_match(text)}))},
            Self::Dotenv(g,n) if Self::dotenv(g.as_ref(),path)=>{let text=std::str::from_utf8(bytes.ok_or("missing-file-data")?).map_err(|_|"invalid-text")?;let mut ids=Vec::new();for line in text.lines(){let t=line.trim();let t=t.strip_prefix("export ").unwrap_or(t);if let Some((k,_))=t.split_once('='){let k=k.trim();if !k.is_empty()&&k.as_bytes().first().is_some_and(|b|b.is_ascii_alphabetic()||*b==b'_')&&k.bytes().all(|b|b.is_ascii_alphanumeric()||b==b'_')&&n.matches(k){ids.push(None);}}}Ok(ids)},
            Self::Dependency(e,n) if providers::applies(e,path)=>Ok(providers::names(e,path,bytes.ok_or("missing-file-data")?)?.into_iter().filter(|s|n.matches(s)).map(|_|None).collect()),_=>Ok(Vec::new())}
    }
}
