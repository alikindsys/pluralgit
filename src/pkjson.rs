// Basically I will omit anything thats not useful for this program
// The actual export is way larger.

use serde::Deserialize;
use serde_with::serde_as;

#[derive(Deserialize, Debug)]
pub struct PkExport {
    version: usize,
    pub name: String,
    pub id: String,
    members: Vec<PkMember>,
}

impl PkExport {
    pub fn match_text(&self, text: String) -> Result<(PkMember, String), String> {
        for member in &self.members {
            for tag in &member.proxy_tags {
                match (&tag.prefix, &tag.suffix) {
                    
                    (None, None) => continue,
                    (None, Some(s)) => if text.ends_with(s.trim()) {
                        return Ok((member.clone(), text.strip_suffix(s.trim()).unwrap().to_owned()));
                    },
                    (Some(p), None) => if text.starts_with(p.trim()) {
                        return Ok((member.clone(), text.strip_prefix(p.trim()).unwrap().to_owned()));
                    },
                    (Some(p), Some(s)) => if text.starts_with(p.trim()) && text.ends_with(s.trim()) {
                        return Ok((member.clone(), text.strip_prefix(p.trim()).unwrap().strip_suffix(s.trim()).unwrap().to_owned()));
                    },
                }
            }
        }
        Err(text.clone())
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct PkMember {
    pub id: String,
    pub name: String,
    proxy_tags: Vec<PkProxyTag>,
}


#[serde_as]
#[derive(Deserialize, Debug, Clone)]
pub struct PkProxyTag {
    #[serde_as(as = "serde_with::NoneAsEmptyString")]
    prefix: Option<String>,
    #[serde_as(as = "serde_with::NoneAsEmptyString")]
    suffix: Option<String>,
}
