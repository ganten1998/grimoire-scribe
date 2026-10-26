#!/bin/sh
# Stands in for kindle/app/update.sh in scenarios (tests/scenarios/u_update_*.qs): check prints what
# `kpm search grimoire` would, the preview channel offering $FAKE_UPDATE_PREVIEW and the stable one
# $FAKE_UPDATE_STABLE, or fails with $FAKE_UPDATE_FAIL as its status (update.sh's 3, 4 or 5).
case "${1:-}" in
check)
    [ -n "${FAKE_UPDATE_FAIL:-}" ] && exit "$FAKE_UPDATE_FAIL"
    sleep 0.3
    case "${2:-}" in preview) v=${FAKE_UPDATE_PREVIEW:-} ;; *) v=${FAKE_UPDATE_STABLE:-} ;; esac
    printf '\033[32mFound 1 package(s) for grimoire:\033[0m\n'
    [ -n "$v" ] && echo "  - grimoire (Grimoire): Handwriting notebooks for the jailbroken Kindle Scribe. $v"
    exit 0
    ;;
esac
exit 2
