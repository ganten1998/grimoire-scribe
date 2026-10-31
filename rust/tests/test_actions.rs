mod testlib;

use grimoire::prelude::*;
use std::collections::HashSet;
use testlib::*;

fn registry_names_groups_and_labels() {
    let old_names: [&str; 36] = [
        "undo",
        "redo",
        "previous-page",
        "next-page",
        "previous-tool",
        "next-tool",
        "white",
        "index",
        "hold-eraser",
        "eraser-mode",
        "clear-page",
        "first-page",
        "last-page",
        "new-page",
        "fit-page",
        "pages",
        "tool-picker",
        "paper",
        "display",
        "pen-feel",
        "fold-panel",
        "library",
        "close-menu",
        "picker-previous",
        "picker-next",
        "add-link",
        "name-page",
        "status-toggle",
        "links",
        "go-to-page",
        "go-back",
        "link-areas",
        "landscape-fit",
        "status-mode",
        "status-corner",
        "gesture-new-page",
    ];
    let expected_group_rows = [10usize, 11, 11, 7, 9, 10, 6];
    check!(ACTION_GROUPS.len() == N_GROUPS && N_GROUPS == expected_group_rows.len());
    check!(SHORTCUT_LAST == 83 && SHORTCUT_GESTURE_NEW_PAGE == 54 && SHORTCUT_STATUS_MODE == 55);
    let mut ids = HashSet::new();
    let mut names = HashSet::new();
    let mut all = Vec::new();
    for id in SHORTCUT_TOOL_BASE..SHORTCUT_TOOL_BASE + TOOL_COUNT as i32 {
        all.push(id);
    }
    for &(name, id) in K_ACTIONS.iter() {
        check!(action_by_name(name) == id);
        all.push(id);
    }
    check!(K_ACTIONS.len() == old_names.len() + 26);
    for name in old_names {
        check!(action_by_name(name) != SHORTCUT_NONE);
    }
    check!(all.len() == 72);
    check!(i18n_set("en"));
    for id in all.iter().copied() {
        let info = action_info(id);
        check!(info.is_some());
        let Some(info) = info else { continue };
        check!(ids.insert(info.id) && names.insert(info.name));
        check!(info.id == id && info.name == shortcut_action_name(id).unwrap_or(""));
        check!(action_by_name(info.name) == id && !info.label.is_empty());
        check!((info.group as usize) < N_GROUPS && action_sources(id) == info.sources);
        check!(action_label(id) != "?" && !action_label(id).is_empty());
        if info.pair != 0 {
            let other = action_info(info.pair);
            check!(other.is_some_and(|p| p.pair == id && p.group == info.group));
        }
    }
    check!(ids.len() == 72 && names.len() == 72);
    for (g, want) in expected_group_rows.iter().enumerate() {
        check!(
            group_row_count(g) == *want
                && group_row_count(g) <= GROUP_ROWS_MAX
                && group_row_count(g) <= 11
        );
        check!(
            ACTION_GROUPS[g].probe.len() > 0
                && ACTION_GROUPS[g].title.len() > 0
                && ACTION_GROUPS[g].label.len() > 0
        );
        for row in 0..*want {
            let learn = learn_row_at(g, row);
            check!(learn.is_some());
            let Some(learn) = learn else { continue };
            check!(learn.act[0] != SHORTCUT_NONE && action_info(learn.act[0]).is_some());
            if learn.act[1] != SHORTCUT_NONE {
                check!(action_info(learn.act[1]).is_some());
            }
        }
    }
    check!(learn_row_at(N_GROUPS, 0).is_none() && learn_row_at(0, usize::MAX).is_none());

    // Every displayed action remains labelable in Spanish, including the shape-assignable set.
    check!(i18n_set("es"));
    for id in all.iter().copied() {
        check!(action_label(id) != "?" && !action_label(id).is_empty());
    }
    i18n_set("en");
}

fn registry_sources_and_pair_rows() {
    let key_pad = SRC_KEY | SRC_PAD | SRC_PEN;
    let touch_shape = SRC_TOUCH | SRC_SHAPE | SRC_PEN;
    check!(action_sources(SHORTCUT_HOLD_ERASER) == key_pad);
    check!(action_sources(SHORTCUT_SHORTCUTS) == touch_shape);
    check!(action_sources(SHORTCUT_BLUETOOTH_CONTROLLER) == touch_shape);
    for &(_, id) in K_ACTIONS.iter() {
        match id {
            SHORTCUT_HOLD_ERASER => {}
            SHORTCUT_PEN_PAN | SHORTCUT_PEN_ZOOM => check!(action_sources(id) == SRC_PEN),
            SHORTCUT_SHORTCUTS | SHORTCUT_BLUETOOTH_CONTROLLER => {}
            _ => check!(action_sources(id) == (SRC_KEY | SRC_PAD | SRC_TOUCH | SRC_SHAPE | SRC_PEN)),
        }
    }
    check!(
        action_sources(SHORTCUT_TOOL_BASE + TOOL_ERASER.0)
            == (SRC_KEY | SRC_PAD | SRC_TOUCH | SRC_SHAPE | SRC_PEN)
    );
    check!(SHORTCUT_SHAPES == 78 && SHORTCUT_SHAPE_HOLD == 79 && SHORTCUT_SHAPE_LAST == 80 && SHORTCUT_INDEX_MAP == 81);
    check!(SHORTCUT_PEN_PAN == 82 && SHORTCUT_PEN_ZOOM == 83);
    for id in (SHORTCUT_TOOL_BASE..SHORTCUT_TOOL_BASE + TOOL_COUNT as i32).chain(K_ACTIONS.iter().map(|(_, id)| *id)) {
        check!(action_sources(id) & SRC_PEN != 0);
    }
    for id in [SHORTCUT_PEN_PAN, SHORTCUT_PEN_ZOOM] {
        check!(action_info(id).is_some_and(|a| a.group as usize == GROUP_PAGES));
        check!(ACTION_GROUPS[GROUP_PAGES].rows.iter().any(|r| r.act == [id, 0]));
    }
    check!(action_info(SHORTCUT_GESTURE_NEW_PAGE).is_some_and(|a| a.group as usize == GROUP_PAGES));
    check!(action_info(SHORTCUT_STATUS_MODE).is_none());
    check!(action_by_name("status-mode") == SHORTCUT_STATUS);
    // The Index list's row is gone: its name is the Graph's, and pages has a row of its own.
    check!(action_info(SHORTCUT_INDEX).is_none() && action_by_name("index") == SHORTCUT_INDEX_MAP);
    check!(shortcut_action_name(SHORTCUT_INDEX_MAP) == Some("index-map") && action_by_name("pages") == SHORTCUT_PAGES);
    check!(!ACTION_GROUPS[GROUP_INDEX].rows.iter().any(|r| r.act[0] == SHORTCUT_INDEX));
    check!(action_info(SHORTCUT_ADD_LINK).is_some_and(|a| a.group as usize == GROUP_INDEX));
    check!(action_info(SHORTCUT_NAME_PAGE).is_some_and(|a| a.group as usize == GROUP_INDEX));
    check!(action_info(SHORTCUT_ZOOM_OUT).is_some_and(|a| a.pair == SHORTCUT_ZOOM_IN));
    check!(action_info(SHORTCUT_LIGHT_DOWN).is_some_and(|a| a.pair == SHORTCUT_LIGHT_UP));
}

fn cached_light_actions_do_not_read_unknown_levels() {
    check!(light_cached() == (-1, -1));
    check!(font_init() == 0);
    ui_init(300.0);
    let root = format!("{}/actions-{}", tl_tmp(), std::process::id());
    check!(store_init(&root) == 0);
    let mut a = App::default();
    a.w = 1860; a.h = 2480;
    settings_default(&mut a.settings);
    for action in [SHORTCUT_LIGHT_DOWN, SHORTCUT_LIGHT_UP, SHORTCUT_WARMTH_DOWN, SHORTCUT_WARMTH_UP] {
        check!(app_global_action(&mut a, action, ActionSrc::Key));
        check!(a.toast == "Light not ready yet." && light_cached() == (-1, -1));
    }
    light_set_intensity(10); light_set_warmth(6);
    check!(app_global_action(&mut a, SHORTCUT_LIGHT_UP, ActionSrc::Key) && light_cached() == (11, 6));
    check!(app_global_action(&mut a, SHORTCUT_WARMTH_DOWN, ActionSrc::Key) && light_cached() == (11, 5));
    check!(std::fs::remove_dir_all(root).is_ok());
}

#[test]
fn test_actions() {
    run!(cached_light_actions_do_not_read_unknown_levels);
    run!(registry_names_groups_and_labels);
    run!(registry_sources_and_pair_rows);
    done();
}
