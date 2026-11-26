use mod_api_stable::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

const MOD_ID: &str = "mlbb_champ_info_tfm2";
const HEROES_MOD_ID: &str = "mlbb_heroes";

const TEXT_ASSET: &str = "asset/base/text/champion";
const DEFAULTS_ASSET: &str = "asset/mlbb_champ_info_tfm2/text/defaults";

// The two cards this mod adds. `passive` is a node the base layout already
// ships but never shows; `attack` is a copy of it that this mod's layout
// override adds. Both sit to the left of skill1.
const CARD_PASSIVE: &str = "passive";
const CARD_ATTACK: &str = "attack";
const SKILL_CARDS: [&str; 3] = ["skill1", "skill2", "ult"];

// The i18n slot each extra card reads: description.<champion>.<FIELD>.
const PASSIVE_FIELD: &str = "passive";
const ATTACK_FIELD: &str = "attack";

const PROBE: &str = "skill1.data.desc";

// Card-relative children. The extra cards are reached by joining the card's
// name, so the same paths work for both.
const CARD_FULL_WIDTH: [&str; 4] = ["data", "data.icon_slot", "data.bar", "data.desc"];
const CHILD_DESC: &str = "data.desc";
const CHILD_ICON: &str = "data.icon_slot.icon";
const CHILD_ICON_BG: &str = "data.icon_slot.icon_bg";
const CHILD_NAME: &str = "data.icon_slot.name";
const CHILD_RANK: &str = "data.icon_slot.text";
const CHILD_COOLTIME: &str = "data.icon_slot.cooltime";

/// Which of the two extra cards the selected champion has text for.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Extras {
    passive: bool,
    attack: bool,
}

impl Extras {
    fn count(self) -> usize {
        self.passive as usize + self.attack as usize
    }
}

// Card row geometry, matching tools/build_layout.py. The row is 1600px wide
// either way: three cards is the base game's own spread, which is what the
// layout file ships and what the screen keeps when there is no attack card;
// four splits the same row into 391s with 12px gaps.
const THREE_CARDS: [(f32, f32); 3] = [(0.0, 527.0), (537.0, 526.0), (1073.0, 527.0)];
const FOUR_CARDS: [(f32, f32); 4] = [
    (0.0, 391.0),
    (403.0, 391.0),
    (806.0, 391.0),
    (1209.0, 391.0),
];
// Five cards is why this fork exists: the row is the same 1600px, so the cards
// narrow to 312s with 10px gaps. Tighter than four, but a passive and an
// attack side by side is worth it.
const FIVE_CARDS: [(f32, f32); 5] = [
    (0.0, 312.0),
    (322.0, 312.0),
    (644.0, 312.0),
    (966.0, 312.0),
    (1288.0, 312.0),
];
const CARD_PADDING: f32 = 20.0;
const COOLTIME_WIDTH: f32 = 78.0;
// Header columns. Three cards keeps the base game's icon/rank/cooldown/name;
// four moves the cooldown to the right edge to leave the narrower card's name
// a middle column of its own. Either way the name ends 192px past the icon.
const NAME_X_WIDE: f32 = 192.0;
const COOLTIME_X_WIDE: f32 = 104.0;
const NAME_X_NARROW: f32 = 104.0;
// Five cards, or any extra card: the rank chip ("Lv. 1") is dropped and the
// name takes the column back, which is the difference between a name that
// wraps to two lines and one that clips after six characters.
const ICON_END: f32 = 40.0;
const NAME_GAP: f32 = 10.0;
const COOLTIME_GAP: f32 = 8.0;

const MAX_DEPTH: usize = 16;

fn join(path: &str, child: &str) -> String {
    if path.is_empty() {
        child.to_string()
    } else {
        format!("{path}.{child}")
    }
}

// The container where the skill cards sit
struct Screen {
    root: String,
}

impl Screen {
    fn node(&self, suffix: &str) -> String {
        join(&self.root, suffix)
    }

    fn discover(ctx: &StableClient<'_>, path: &str, depth: usize) -> Option<Self> {
        if depth > MAX_DEPTH {
            return None;
        }
        let children = ctx.ui_child_names(path);
        if children.iter().any(|child| child == CARD_PASSIVE) {
            let screen = Self { root: path.into() };
            if [CARD_PASSIVE, CARD_ATTACK, SKILL_CARDS[0]]
                .iter()
                .all(|card| ctx.ui_exists(&screen.desc(card)))
            {
                return Some(screen);
            }
        }
        children
            .iter()
            .find_map(|child| Self::discover(ctx, &join(path, child), depth + 1))
    }

    fn desc(&self, card: &str) -> String {
        self.node(&join(card, CHILD_DESC))
    }

    /// Show whichever extra cards the champion has text for and re-spread the
    /// row to suit: three cards with neither, four with one, five with both.
    ///
    /// Hidden is the layout file's own state, so the screen starts out as the
    /// base game's three cards and only widens once a champion turns up with
    /// passive or attack text of its own.
    fn arrange(&self, ctx: &mut StableClient<'_>, extras: Extras) {
        // Anything the champion has no text for stays hidden, and takes no
        // part in the row.
        for (card, shown) in [(CARD_PASSIVE, extras.passive), (CARD_ATTACK, extras.attack)] {
            ctx.ui_set_properties(&self.node(card), &format!("visible: {shown};"));
        }

        let n = SKILL_CARDS.len() + extras.count();
        let frames: &[(f32, f32)] = match n {
            3 => &THREE_CARDS,
            4 => &FOUR_CARDS,
            _ => &FIVE_CARDS,
        };
        let five = n >= 5;

        let mut cards: Vec<&str> = Vec::with_capacity(n);
        if extras.passive {
            cards.push(CARD_PASSIVE);
        }
        if extras.attack {
            cards.push(CARD_ATTACK);
        }
        cards.extend_from_slice(&SKILL_CARDS);

        for (card, &(x, width)) in cards.iter().zip(frames) {
            let is_extra = *card == CARD_PASSIVE || *card == CARD_ATTACK;
            ctx.ui_set_properties(&self.node(card), &format!("x: {x}px; width: {width}px;"));

            let body = width - CARD_PADDING;
            for child in CARD_FULL_WIDTH {
                let props = format!("width: {body}px;");
                ctx.ui_set_properties(&self.node(&join(card, child)), &props);
            }

            // A passive and a basic attack are not levelled, so the rank chip
            // is meaningless on them at any width. At five wide there is no
            // room for it on the skills either, and the name needs the column.
            let show_rank = !(five || is_extra);
            ctx.ui_set_properties(
                &self.node(&join(card, CHILD_RANK)),
                &format!("visible: {show_rank};"),
            );

            let name_x = if show_rank {
                if n == 4 {
                    NAME_X_NARROW
                } else {
                    NAME_X_WIDE
                }
            } else {
                ICON_END + NAME_GAP
            };
            let cooltime = self.node(&join(card, CHILD_COOLTIME));
            let has_cooltime = ctx.ui_exists(&cooltime);
            // At three wide the cooldown sits between the rank chip and the
            // name; from four wide it moves to the right edge.
            let trailing = if has_cooltime && n >= 4 {
                COOLTIME_WIDTH + COOLTIME_GAP
            } else {
                0.0
            };
            let name_width = body - name_x - trailing;
            let props = format!("x: {name_x}px; width: {name_width}px;");
            ctx.ui_set_properties(&self.node(&join(card, CHILD_NAME)), &props);

            if has_cooltime {
                let cooltime_x = if n == 3 {
                    COOLTIME_X_WIDE
                } else {
                    body - COOLTIME_WIDTH
                };
                ctx.ui_set_properties(&cooltime, &format!("x: {cooltime_x}px;"));
            }
        }
    }
}

const SEARCH_INTERVAL_FRAMES: u32 = 30;

#[derive(Default)]
struct State {
    screen: Option<Screen>,
    search_wait: u32,
    by_skill_text: Vec<(Pattern, String)>,
    indexed: bool,
    /// Keyed by (champion, slot) where slot is "passive" or "attack".
    icons: HashMap<(String, String), String>,
    written_passive: String,
    written_attack: String,
    selection: String,
    /// Which extra cards the row is currently laid out for. None is the
    /// layout file's own state, so a fresh screen needs no work.
    arranged: Option<Extras>,
}

struct ChampInfoCards {
    state: Mutex<State>,
}

fn strip_markup(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut depth = 0usize;
    for ch in text.chars() {
        match ch {
            '<' => depth += 1,
            '>' => depth = depth.saturating_sub(1),
            _ if depth == 0 => out.push(ch),
            _ => {}
        }
    }
    out
}

struct Pattern {
    parts: Vec<String>,
    anchor_start: bool,
    anchor_end: bool,
}

impl Pattern {
    fn new(template: &str) -> Self {
        let mut parts: Vec<String> = Vec::new();
        let mut literal = String::new();
        let mut depth = 0usize;
        for ch in template.chars() {
            match ch {
                '{' => {
                    depth += 1;
                    if depth == 1 && !literal.is_empty() {
                        parts.push(std::mem::take(&mut literal));
                    }
                }
                '}' => depth = depth.saturating_sub(1),
                _ if depth == 0 => literal.push(ch),
                _ => {}
            }
        }
        let anchor_end = !literal.is_empty();
        if anchor_end {
            parts.push(literal);
        }
        Self {
            anchor_start: !template.starts_with('{'),
            anchor_end,
            parts,
        }
    }

    fn matches(&self, text: &str) -> bool {
        if self.parts.is_empty() {
            return false;
        }
        let mut tail = text;
        for (index, part) in self.parts.iter().enumerate() {
            let first = index == 0;
            let last = index + 1 == self.parts.len();
            if last && self.anchor_end {
                let Some(head) = tail.strip_suffix(part.as_str()) else {
                    return false;
                };
                if first && self.anchor_start && !head.is_empty() {
                    return false;
                }
                tail = "";
            } else if first && self.anchor_start {
                match tail.strip_prefix(part.as_str()) {
                    Some(rest) => tail = rest,
                    None => return false,
                }
            } else {
                match tail.find(part.as_str()) {
                    Some(at) => tail = &tail[at + part.len()..],
                    None => return false,
                }
            }
        }
        true
    }
}

/// Steam app id for Teamfight Manager 2, used to locate subscribed Workshop mods.
const STEAM_APP_ID: &str = "3009300";

/// The directories that hold installed mods, in preference order.
///
/// A mod reaches the game one of two ways, and the icons have to be found in
/// both. A local build is deployed to `<game>/mods/<mod id>`. A Workshop
/// subscription is downloaded by Steam to
/// `<library>/steamapps/workshop/content/<app id>/<published file id>` and left
/// there - it is never copied into `<game>/mods`, and its folder is named by
/// number rather than by mod id.
fn mod_roots() -> Vec<PathBuf> {
    let Some(game) = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
    else {
        return Vec::new();
    };

    let mut roots = vec![game.join("mods")];
    // `<library>/steamapps/common/<game>` -> `<library>/steamapps/workshop/...`
    if let Some(steamapps) = game.parent().and_then(Path::parent) {
        roots.push(
            steamapps
                .join("workshop")
                .join("content")
                .join(STEAM_APP_ID),
        );
    }
    roots
}

/// The mod id owning `dir`, which is what asset paths are keyed by.
///
/// A Workshop folder is a published file id, so the id has to be read out of the
/// manifest inside it. Deployed folders are already named after the mod, which is
/// the fallback when there is no readable manifest.
fn mod_id_at(dir: &Path) -> Option<String> {
    let manifest = std::fs::read_to_string(dir.join("mod.mod_info")).ok()?;
    let id = top_level_string(&manifest, "mod_id")?;
    (!id.is_empty()).then_some(id)
}

/// Read the JSON string opening at `open`, plus the index just past its close.
fn read_string(json: &str, open: usize) -> Option<(&str, usize)> {
    let bytes = json.as_bytes();
    let mut i = open + 1;
    while i < bytes.len() {
        match bytes[i] {
            // Escapes are skipped, not decoded - nothing read out of a manifest
            // needs them, and this keeps a `\"` from ending the string early.
            b'\\' => i += 2,
            b'"' => return Some((&json[open + 1..i], i + 1)),
            _ => i += 1,
        }
    }
    None
}

/// Pull one string field off the outermost object, without a JSON dependency.
///
/// Depth is the whole point: `mod.mod_info` lists dependencies as objects that
/// each carry their own `mod_id`, and those are written before the manifest's
/// own, so a plain substring search finds a dependency - usually `base` - first.
fn top_level_string(json: &str, key: &str) -> Option<String> {
    let bytes = json.as_bytes();
    let mut i = 0;
    let mut depth = 0usize;
    let mut last_key = "";
    while i < bytes.len() {
        match bytes[i] {
            b'{' | b'[' => {
                depth += 1;
                i += 1;
            }
            b'}' | b']' => {
                depth = depth.saturating_sub(1);
                i += 1;
            }
            b'"' => {
                let (text, next) = read_string(json, i)?;
                i = next;
                // A `:` after it makes it a key; otherwise it is the value of
                // whichever key came last.
                let colon =
                    bytes[i..].iter().find(|byte| !byte.is_ascii_whitespace()) == Some(&b':');
                if colon {
                    last_key = text;
                } else if depth == 1 && last_key == key {
                    return Some(text.to_string());
                }
            }
            _ => i += 1,
        }
    }
    None
}

/// The image each card looks for, paired with the slot it fills.
const ICON_SUFFIXES: [(&str, &str); 2] = [
    (PASSIVE_FIELD, "_passive.png"),
    (ATTACK_FIELD, "_base_attack.png"),
];

type IconKey = (String, String);

/// Prefer the current hero pack, then this companion's copies, then other
/// installed mods. Give local installs precedence within each source.
fn icon_index(roots: &[PathBuf]) -> HashMap<IconKey, String> {
    let mut icons = HashMap::new();
    for (root_order, root) in roots.iter().enumerate() {
        for entry in std::fs::read_dir(root).into_iter().flatten().flatten() {
            let dir = entry.path();
            let Some(mod_id) =
                mod_id_at(&dir).or_else(|| entry.file_name().to_str().map(str::to_string))
            else {
                continue;
            };
            let mod_order = match mod_id.as_str() {
                HEROES_MOD_ID => 0,
                MOD_ID => 1,
                _ => 2,
            };
            for icon in std::fs::read_dir(dir.join("icons"))
                .into_iter()
                .flatten()
                .flatten()
            {
                let filename = icon.file_name();
                let Some(name) = filename.to_str() else {
                    continue;
                };
                let canonical = ICON_SUFFIXES.iter().find_map(|&(slot, suffix)| {
                    name.strip_suffix(suffix).map(|id| (id, slot, false))
                });
                let alternate = || {
                    if mod_id == HEROES_MOD_ID {
                        name.strip_suffix("_attack.png")
                            .map(|id| (id, ATTACK_FIELD, true))
                    } else {
                        None
                    }
                };
                let Some((champion, slot, legacy)) = canonical.or_else(alternate) else {
                    continue;
                };
                if champion.is_empty() || !icon.path().is_file() {
                    continue;
                }
                let key = (champion.to_string(), slot.to_string());
                let stem = name.trim_end_matches(".png");
                let candidate = (
                    mod_order,
                    root_order,
                    legacy,
                    format!("asset/{mod_id}/icons/{stem}"),
                );
                let preferred = icons.entry(key).or_insert_with(|| candidate.clone());
                if candidate < *preferred {
                    *preferred = candidate;
                }
            }
        }
    }
    icons
        .into_iter()
        .map(|(key, (_, _, _, asset))| (key, asset))
        .collect()
}

impl ChampInfoCards {
    fn description(ctx: &StableClient<'_>, champion: &str, field: &str) -> Option<String> {
        Self::text(ctx, &format!("description.{champion}.{field}"))
    }

    fn text(ctx: &StableClient<'_>, key: &str) -> Option<String> {
        Self::text_in(ctx, TEXT_ASSET, key)
    }

    fn default_text(ctx: &StableClient<'_>, key: &str) -> Option<String> {
        Self::text_in(ctx, DEFAULTS_ASSET, key)
    }

    fn text_in(ctx: &StableClient<'_>, asset: &str, key: &str) -> Option<String> {
        let key = format!("{asset}?{key}");
        ctx.i18n(&key)
            .or_else(|| ctx.i18n(&format!("#{key}")))
            .filter(|text| !text.trim().is_empty())
    }

    /// The body text for one of the two extra cards.
    ///
    /// Read straight out of i18n rather than through the champion's data block,
    /// which is the whole point: a passive block carries no description
    /// pointer, so the game never renders this text on its own even though the
    /// localisations have had it all along.
    fn card_text(ctx: &StableClient<'_>, champion: &str, field: &str) -> Option<String> {
        Self::description(ctx, champion, field)
    }

    /// The heading for one of the two extra cards.
    fn card_name(ctx: &StableClient<'_>, champion: &str, field: &str) -> String {
        Self::text(ctx, &format!("skill_name.{champion}.{field}"))
            .or_else(|| {
                // These titles occur in the shipped descriptions, but v3.1.0
                // has no separate title key for the companion to look up.
                match (champion, field) {
                    ("mlbb_sun", PASSIVE_FIELD) => Some("Simian God".into()),
                    ("mlbb_xborg", PASSIVE_FIELD) => Some("Firaga Guard".into()),
                    _ => None,
                }
            })
            .or_else(|| Self::default_text(ctx, &format!("{field}_name")))
            .or_else(|| {
                (field == ATTACK_FIELD)
                    .then(|| Self::text(ctx, "stat.attack"))
                    .flatten()
            })
            .unwrap_or_else(|| {
                if field == ATTACK_FIELD {
                    "Basic Attack".to_string()
                } else {
                    "Passive".to_string()
                }
            })
    }

    fn build_index(state: &mut State, ctx: &StableClient<'_>) {
        if state.indexed {
            return;
        }
        let roster = ctx.champion_names();
        if roster.is_empty() {
            return;
        }
        state.indexed = true;
        state.icons = icon_index(&mod_roots());
        state.by_skill_text = roster
            .into_iter()
            .filter_map(|champion| {
                let skill = Self::description(ctx, &champion, "skill")?;
                Some((Pattern::new(&strip_markup(&skill)), champion))
            })
            .collect();
    }

    fn selected(state: &State, shown: &str) -> Option<String> {
        if shown.is_empty() {
            return None;
        }
        let mut exact = state
            .by_skill_text
            .iter()
            .filter(|(skill, _)| skill.anchor_start && skill.anchor_end && skill.parts == [shown]);
        if let Some((_, champion)) = exact.next() {
            return exact.next().is_none().then(|| champion.clone());
        }
        let mut hits = state
            .by_skill_text
            .iter()
            .filter(|(skill, _)| skill.matches(shown));
        let (_, champion) = hits.next()?;
        hits.next().is_none().then(|| champion.clone())
    }
}

impl StableExtension for ChampInfoCards {
    fn on_init(&self, ctx: &mut StableClient<'_>) {
        if let Ok(mut state) = self.state.lock() {
            Self::build_index(&mut state, ctx);
        }
    }

    fn post_update(&self, ctx: &mut StableClient<'_>, _dt_micros: u64) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };

        let Some(screen) = state.screen.take() else {
            match state.search_wait.checked_sub(1) {
                Some(remaining) => state.search_wait = remaining,
                None => {
                    state.search_wait = SEARCH_INTERVAL_FRAMES;
                    state.screen = Screen::discover(ctx, "", 0);
                }
            }
            return;
        };
        if !ctx.ui_exists(&screen.desc(CARD_PASSIVE)) {
            state.written_passive.clear();
            state.written_attack.clear();
            state.selection.clear();
            state.arranged = None;
            state.search_wait = 0;
            return;
        }

        Self::build_index(&mut state, ctx);

        let shown = ctx
            .ui_text(&screen.node(PROBE))
            .map(|text| strip_markup(&text))
            .unwrap_or_default();
        let found = Self::selected(&state, &shown);

        let passive = found
            .as_ref()
            .and_then(|champion| Self::card_text(ctx, champion, PASSIVE_FIELD));
        let attack = found
            .as_ref()
            .and_then(|champion| Self::card_text(ctx, champion, ATTACK_FIELD));

        // Only champions that ship their own passive or attack text get those
        // cards; anything missing is dropped and the rest of the row spreads
        // to fill the space, so the screen is never left with a blank card.
        let extras = Extras {
            passive: passive.is_some(),
            attack: attack.is_some(),
        };
        if state.arranged != Some(extras) {
            screen.arrange(ctx, extras);
            state.arranged = Some(extras);
        }

        let passive = passive.unwrap_or_default();
        if state.written_passive != passive {
            ctx.ui_set_text(&screen.desc(CARD_PASSIVE), &passive);
            state.written_passive = passive;
        }

        let attack = attack.unwrap_or_default();
        if state.written_attack != attack {
            ctx.ui_set_text(&screen.desc(CARD_ATTACK), &attack);
            state.written_attack = attack;
        }

        if state.selection != shown {
            let champion = found.unwrap_or_default();
            for (card, field) in [(CARD_PASSIVE, PASSIVE_FIELD), (CARD_ATTACK, ATTACK_FIELD)] {
                let key = (champion.clone(), field.to_string());
                let icon = state.icons.get(&key).cloned().unwrap_or_default();
                for child in [CHILD_ICON, CHILD_ICON_BG] {
                    let props = format!("visible: {};", !icon.is_empty());
                    ctx.ui_set_properties(&screen.node(&join(card, child)), &props);
                }
                if !icon.is_empty() {
                    let props = format!("source: \"{icon}\";");
                    ctx.ui_set_properties(&screen.node(&join(card, CHILD_ICON)), &props);
                }

                let name = Self::card_name(ctx, &champion, field);
                ctx.ui_set_text(&screen.node(&join(card, CHILD_NAME)), &name);
            }

            state.selection = shown;
        }

        state.screen = Some(screen);
    }
}

fn init(host: &StableHost) -> StableMod {
    host.log(
        LogLevel::Info,
        "mlbb_champ_info_tfm2: passive and basic attack card extension registering",
    );
    let mut reg = StableMod::new(MOD_ID);
    reg.set_extension(ChampInfoCards {
        state: Mutex::new(State::default()),
    });
    reg
}

declare_stable_mod!(init);

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::ffi::c_void;
    use std::mem::size_of;
    use std::ptr::{copy_nonoverlapping, null};
    use std::time::{SystemTime, UNIX_EPOCH};

    /// The manifest that broke the Workshop icon lookup: `dependencies` carries
    /// its own `mod_id`, and it is serialised before the manifest's own.
    const MANIFEST: &str = r#"{
  "author": "shirograhm",
  "dependencies": [
    {
      "mod_id": "base",
      "version": ">=0.5.0"
    }
  ],
  "description": "Adds characters from Avatar: The Last Airbender to Teamfight Manager 2.",
  "mod_id": "avatar_characters_tfm2",
  "name": "Avatar Characters",
  "version": "0.1.1"
}"#;

    #[test]
    fn reads_past_a_nested_mod_id() {
        assert_eq!(
            top_level_string(MANIFEST, "mod_id").as_deref(),
            Some("avatar_characters_tfm2")
        );
    }

    #[test]
    fn reads_a_mod_id_written_before_dependencies() {
        let manifest = r#"{"mod_id": "champ_attack_info_tfm2",
          "dependencies": [{"mod_id": "base"}]}"#;
        assert_eq!(
            top_level_string(manifest, "mod_id").as_deref(),
            Some("champ_attack_info_tfm2")
        );
    }

    #[test]
    fn missing_key_is_none() {
        assert_eq!(top_level_string(r#"{"name": "x"}"#, "mod_id"), None);
    }

    #[test]
    fn shared_skill_text_cannot_claim_someone_elses_cards() {
        let mut state = State {
            by_skill_text: vec![
                (Pattern::new("Deal damage."), "first".into()),
                (Pattern::new("Deal damage."), "second".into()),
                (Pattern::new("Deal {value} damage."), "third".into()),
            ],
            ..State::default()
        };
        assert_eq!(ChampInfoCards::selected(&state, "Deal damage."), None);
        assert_eq!(
            ChampInfoCards::selected(&state, "Deal 150 damage.").as_deref(),
            Some("third")
        );
        state.by_skill_text.remove(1);
        assert_eq!(
            ChampInfoCards::selected(&state, "Deal damage.").as_deref(),
            Some("first")
        );
    }

    #[test]
    fn icons_choose_matching_hero_pack_then_companion_then_other_mod() {
        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("champ-info-{time}"));
        let local = dir.join("local");
        let workshop = dir.join("workshop");
        let write_icons = |path: &Path, id: &str, names: &[&str]| {
            let icon_dir = path.join("icons");
            std::fs::create_dir_all(&icon_dir).unwrap();
            std::fs::write(path.join("mod.mod_info"), format!(r#"{{"mod_id":"{id}"}}"#)).unwrap();
            for name in names {
                std::fs::write(icon_dir.join(name), []).unwrap();
            }
        };
        write_icons(
            &local.join("compatibility"),
            "other_mod",
            &[
                "mlbb_xborg_passive.png",
                "guest_passive.png",
                "guest_attack.png",
            ],
        );
        write_icons(
            &local.join(MOD_ID),
            MOD_ID,
            &[
                "mlbb_sun_passive.png",
                "mlbb_xborg_passive.png",
                "mlbb_xborg_base_attack.png",
            ],
        );
        write_icons(
            &workshop.join("12345"),
            HEROES_MOD_ID,
            &["mlbb_xborg_passive.png", "mlbb_xborg_attack.png"],
        );
        let icons = icon_index(&[local, workshop]);
        std::fs::remove_dir_all(dir).unwrap();

        let asset = |champion: &str, kind: &str| {
            icons
                .get(&(champion.into(), kind.into()))
                .map(String::as_str)
        };
        assert_eq!(
            asset("mlbb_xborg", PASSIVE_FIELD),
            Some("asset/mlbb_heroes/icons/mlbb_xborg_passive"),
        );
        assert_eq!(
            asset("mlbb_xborg", ATTACK_FIELD),
            Some("asset/mlbb_heroes/icons/mlbb_xborg_attack"),
        );
        assert_eq!(
            asset("mlbb_sun", PASSIVE_FIELD),
            Some("asset/mlbb_champ_info_tfm2/icons/mlbb_sun_passive"),
        );
        assert_eq!(
            asset("guest", PASSIVE_FIELD),
            Some("asset/other_mod/icons/guest_passive")
        );
        assert_eq!(asset("guest", ATTACK_FIELD), None);
    }

    struct MockUi {
        screen: Cell<&'static str>,
    }

    impl MockUi {
        fn children(&self, path: &str) -> Vec<&str> {
            match path {
                "" => vec!["unrelated", self.screen.get()],
                "unrelated" => vec![CARD_PASSIVE],
                p if p == self.screen.get() => vec![CARD_PASSIVE, CARD_ATTACK, "skill1"],
                _ => Vec::new(),
            }
        }
    }

    unsafe fn path_from_raw(path: *const u8, len: usize) -> String {
        String::from_utf8_lossy(std::slice::from_raw_parts(path, len)).into_owned()
    }

    unsafe extern "C" fn exists(state: *const c_void, path: *const u8, len: usize) -> bool {
        let ui = &*state.cast::<MockUi>();
        let path = path_from_raw(path, len);
        path == "unrelated.passive.data.desc"
            || [CARD_PASSIVE, CARD_ATTACK, "skill1"]
                .iter()
                .any(|card| path == format!("{}.{card}.data.desc", ui.screen.get()))
    }

    unsafe extern "C" fn child_count(
        state: *const c_void,
        path: *const u8,
        len: usize,
        count: *mut usize,
    ) -> bool {
        let ui = &*state.cast::<MockUi>();
        *count = ui.children(&path_from_raw(path, len)).len();
        true
    }

    unsafe extern "C" fn child_name(
        state: *const c_void,
        path: *const u8,
        len: usize,
        index: usize,
        buf: *mut u8,
        cap: usize,
        written: *mut usize,
    ) -> bool {
        let ui = &*state.cast::<MockUi>();
        let names = ui.children(&path_from_raw(path, len));
        let Some(name) = names.get(index) else {
            return false;
        };
        *written = name.len();
        if !buf.is_null() {
            if cap < name.len() {
                return false;
            }
            copy_nonoverlapping(name.as_ptr(), buf, name.len());
        }
        true
    }

    #[test]
    fn finds_the_card_row_again_after_the_screen_moves() {
        // All slots in the SDK table are optional function pointers except size.
        let mut ui: UiVtableV1 = unsafe { std::mem::zeroed() };
        ui.size = size_of::<UiVtableV1>();
        ui.query_exists = Some(exists);
        ui.child_count = Some(child_count);
        ui.child_name_at = Some(child_name);
        let mock = MockUi {
            screen: Cell::new("first"),
        };
        let mut raw = ClientCtxV1 {
            size: size_of::<ClientCtxV1>(),
            ui: &ui,
            scene: null(),
            asset: null(),
            save: null(),
            net: null(),
            state: std::ptr::from_ref(&mock).cast_mut().cast(),
            data: null(),
            draw: null(),
        };
        let extension = ChampInfoCards {
            state: Mutex::new(State::default()),
        };
        let mut update = || {
            // The mock and table stay live for this simulated callback.
            let mut client = unsafe { StableClient::from_raw(&mut raw).unwrap() };
            extension.post_update(&mut client, 0);
            let state = extension.state.lock().unwrap();
            state.screen.as_ref().map(|screen| screen.root.clone())
        };
        assert_eq!(update().as_deref(), Some("first"));
        mock.screen.set("reopened");
        assert_eq!(update(), None);
        assert_eq!(update().as_deref(), Some("reopened"));
    }
}
