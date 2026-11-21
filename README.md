# MLBB Champ Info Cards: Passive + Basic Attack

Adds **passive** and **basic attack** cards to the Champ Info screen, to the left
of Skill 1 — five cards across: Passive, Basic Attack, Skill 1, Skill 2, Ultimate.

A fork of [champ_attack_info_tfm2](https://github.com/shirograhm/champ_attack_info_tfm2)
by **shirograhm** (MIT licence), which added the fourth (basic attack) card.

## Why the game shows no passive text

Teamfight Manager 2's Champ Info screen renders three cards: Skill 1, Skill 2 and
the Ultimate. There is a `passive` node in the layout, but no champion's passive
data block carries a `description` pointer, so the game never fills it — even
though the localisation files have had the passive text the whole time.

This mod reads the text straight out of i18n (`description.<champion>.passive`)
instead of going through the data block, so the text that was already written
finally gets shown.

## What it shows

| Champion has… | Cards shown |
|---|---|
| both passive and attack text | **5** — passive, attack, skill 1, skill 2, ult |
| only attack text | **4** — attack, skill 1, skill 2, ult |
| only passive text | **4** — passive, skill 1, skill 2, ult |
| neither | **3** — the base game's own row, untouched |

The row is always the same 1600 px; the cards narrow to fit. At five wide the
"Lv. 1" rank chip is dropped so the card name keeps a readable column.

## For modders

Add a `passive` key alongside your existing `attack`, `skill`, `skill2` and `ult`
descriptions:

```json
"description": {
  "your_champion": {
    "passive": "Every 3 hits, the next attack deals bonus true damage...",
    "attack":  "Basic attacks deal 100% AD physical damage...",
    "skill":   "..."
  }
}
```

Optional name and icon:

```json
"skill_name": { "your_champion": { "passive": "Mystic Favor" } }
```

Icons are looked up as `icons/<champion_id>_passive.png` and
`icons/<champion_id>_base_attack.png`, where `champion_id` is the champion's own
id — for the MLBB roster that includes the `mlbb_` prefix
(`mlbb_belerick_passive.png`).

## Licence

MIT, as the original. See `LICENSE`.

## v0.2.5

Adds Yu Zhong’s basic attack and Cursing Touch icons, bringing coverage to all 27 heroes in MLBB Heroes v2.8.0. Descriptions and values follow the installed MLBB Heroes version.

## v0.2.4

Includes passive and basic attack icons for all 26 heroes in MLBB Heroes v2.7.0. Eudora, Julian, Popol & Kupa, Granger, Hanzo and Atlas are new to this companion release. Card descriptions come from the installed hero mod so values and localization follow that mod.

### Install on Windows

Use Teamfight Manager 2 0.7.3 or newer. Close the game and extract the release ZIP into the game's `mods` directory. The mod's manifest should be at `mods/mlbb_champ_info_tfm2/mod.mod_info`. Enable the companion alongside MLBB Heroes. Open a hero's Champ Info to see the five cards.

### Build from source

Copy the game's `mod-sdk-stable/mod-api-stable` directory into this project's `sdk/` directory. Run `cargo test --locked`, then `cargo build --release --locked` on Windows. Place the resulting `mlbb_champ_info_tfm2.dll` beside `mod.mod_info`.

Passive art for Eudora, Julian, Granger and Atlas is reused from the MLBB Heroes project. Hanzo's mark icon and Popol & Kupa's paw icon are original graphics for this companion update. The new basic attack icons use attack frames from the MLBB Heroes sprite sheets, with Eudora's existing icon reused. Mobile Legends artwork belongs to its original rights holders.
