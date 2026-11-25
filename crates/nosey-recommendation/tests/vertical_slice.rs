//! Test-first acceptance cases for the synthetic project-wide vertical slice.
#![forbid(unsafe_code)]
use nosey_registry::{CatalogIndex,RegistryInput,compile,parse_catalog};
use nosey_scanner::{Completeness,Settings,VersionKind,MatchKind,scan};
use nosey_recommendation::{evaluate,removal_candidates,Group};
use std::{collections::{BTreeMap,BTreeSet},path::Path,sync::atomic::AtomicBool};
#[test]
fn aggregate_and_recommend() -> Result<(),Box<dyn std::error::Error>> {
    let root=Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/nosey");
    let one=compile(&root.join("synthetic-registry"))?;
    let two=compile(&root.join("synthetic-registry"))?;
    assert_eq!(one,two);
    let catalog=parse_catalog(&one)?;
    let index=CatalogIndex::build(vec![RegistryInput{id:"synthetic".into(),priority:1,approved:true,enabled:true,catalog}],"org.example.nosey.intelligence")?;
    let mut scan=scan(&root.join("project"),&index,&Settings::default(),&AtomicBool::new(false))?;
    assert_eq!(scan.completeness,Completeness::Complete);
    assert_eq!(scan.technologies.len(),2);
    assert!(scan.technologies.values().flatten().any(|e| e.match_kind == Some(MatchKind::Dependency)
        && e.identifier.is_some() && e.version.as_ref().is_some_and(|v|v.kind==VersionKind::ManifestConstraint)));
    let dismissed=BTreeSet::from(["urn:example:skill:alpha".into()]);
    let targets=BTreeSet::from(["agent-a".into()]);
    let rec=evaluate(&index,&scan,&dismissed,&targets,&BTreeMap::new());
    assert_eq!(rec.recommendations.len(),3);
    assert_eq!(rec.recommendations.first().map(|r|r.id.as_str()),Some("urn:example:skill:alpha"));
    assert_eq!(rec.recommendations.first().map(|r|r.group),Some(Group::Dismissed));
    assert_eq!(rec.recommendations.first().map(|r|r.reasons.len()),Some(2));
    assert_eq!(rec.recommendations.get(1).map(|r|r.id.as_str()),Some("urn:example:skill:combined"));
    let former=BTreeSet::from(["urn:example:skill:old".into()]);
    assert_eq!(removal_candidates(&scan,true,true,&former,&rec),former);
    scan.completeness=Completeness::Incomplete;
    assert!(removal_candidates(&scan,true,true,&former,&rec).is_empty());
    Ok(())
}
