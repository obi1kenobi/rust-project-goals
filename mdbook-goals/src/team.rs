use std::{collections::BTreeMap, sync::OnceLock};

use rust_team_data::v1;
use serde::de::DeserializeOwned;

trait Load<T> {
    fn load(&self, op: impl FnOnce() -> anyhow::Result<T>) -> anyhow::Result<&T>;
}

impl<T> Load<T> for OnceLock<anyhow::Result<T>> {
    fn load(&self, op: impl FnOnce() -> anyhow::Result<T>) -> anyhow::Result<&T> {
        match self.get_or_init(op) {
            Ok(data) => Ok(data),
            Err(e) => Err(anyhow::anyhow!("failed to fetch: {e:?}")),
        }
    }
}

/// Given a username like `@foo` finds the corresponding person data (if any).
pub fn get_person_data(username: &str) -> anyhow::Result<Option<&'static v1::Person>> {
    static DATA: OnceLock<anyhow::Result<BTreeMap<String, v1::Person>>> = OnceLock::new();
    let people = DATA.load(|| {
        let data: v1::People = fetch("people.json")?;
        Ok(data
            .people
            .into_iter()
            .map(|(username, value)| (username.to_lowercase(), value))
            .collect())
    })?;

    Ok(people.get(&username[1..].to_lowercase()))
}

#[derive(Debug, PartialOrd, Ord, PartialEq, Eq)]
pub struct TeamName(String);

impl std::fmt::Display for TeamName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}]", self.0)
    }
}

pub fn get_team_names() -> anyhow::Result<impl Iterator<Item = &'static TeamName>> {
    Ok(get_teams()?.keys())
}

fn get_teams() -> anyhow::Result<&'static BTreeMap<TeamName, v1::Team>> {
    static DATA: OnceLock<anyhow::Result<BTreeMap<TeamName, v1::Team>>> = OnceLock::new();
    DATA.load(|| {
        let teams: v1::Teams = fetch("teams.json")?;
        Ok(teams
            .teams
            .into_iter()
            .map(|(team_name, value)| (TeamName(team_name.to_lowercase()), value))
            .collect())
    })
}

pub fn get_team_name(team_name: &str) -> anyhow::Result<Option<&'static TeamName>> {
    let team_name = TeamName(team_name.to_string());
    Ok(get_teams()?.get_key_value(&team_name).map(|(key, _)| key))
}

impl TeamName {
    /// Get the data for this team.
    pub fn data(&self) -> &'static v1::Team {
        get_teams().unwrap().get(self).unwrap()
    }

    pub fn url(&self) -> String {
        if let Some(website) = &self.data().website_data {
            if let Some(url) = &website.repo {
                return url.to_string();
            }
        }

        // FIXME: do better :)
        format!("https://www.rust-lang.org/governance/teams")
    }
}

fn fetch<T>(path: &str) -> anyhow::Result<T>
where
    T: DeserializeOwned,
{
    let url = format!("{}/{}", v1::BASE_URL, path);
    Ok(reqwest::blocking::get(&url)?.json()?)
}
