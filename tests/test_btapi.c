// test_btapi.c: the bounded JSON reader, the Bluetooth helper's answers read from it (a /status the
// helper gave on the Kindle, its address made up), and the client against tests/fake_bthelper.py.
#include "bluetooth.h"
#include "common.h"
#include "json_min.h"
#include "testlib.h"

#include <signal.h>
#include <sys/wait.h>
#include <unistd.h>

TEST(json_reads_what_it_should_and_refuses_the_rest) {
    const char *s = "{\"ok\": true, \"version\": \"3.18.0\", \"n\": -42,"
                    " \"devices\": [{\"address\": \"AA:BB\", \"name\": \"Pro \\u00e9\\ud83c\\udfb5\"},"
                    " {\"address\": \"CC\", \"name\": null}], \"empty\": {}, \"arr\": []}";
    JsonTok t[64];
    int n = json_parse(s, strlen(s), t, 64);
    CHECK(n > 0 && t[0].type == JS_OBJECT && t[0].size == 6);
    bool b;
    int v;
    char out[64];
    CHECK(json_bool(&t[json_get(s, t, n, 0, "ok")], &b) && b);
    CHECK(json_string(s, &t[json_get(s, t, n, 0, "version")], out, sizeof out) && !strcmp(out, "3.18.0"));
    CHECK(json_int(s, &t[json_get(s, t, n, 0, "n")], &v) && v == -42);
    int devs = json_get(s, t, n, 0, "devices");
    CHECK(devs > 0 && t[devs].size == 2);
    int d0 = json_item(t, n, devs, 0), d1 = json_item(t, n, devs, 1);
    CHECK(json_string(s, &t[json_get(s, t, n, d0, "name")], out, sizeof out) && !strcmp(out, "Pro \xc3\xa9\xf0\x9f\x8e\xb5"));
    CHECK(json_string(s, &t[json_get(s, t, n, d0, "name")], out, 7) && !strcmp(out, "Pro \xc3\xa9"));  // cut whole
    CHECK(t[json_get(s, t, n, d1, "name")].type == JS_NULL);
    CHECK(json_get(s, t, n, 0, "missing") == -1 && json_item(t, n, devs, 2) == -1);
    CHECK(json_get(s, t, n, json_get(s, t, n, 0, "arr"), "x") == -1);
    const char *bad[] = {"", "{", "{\"a\":}", "[1,]", "{\"a\" 1}", "\"\\x\"", "tru", "01x", "{\"a\":1} 2",
                         "[\"\x01\"]", "-", "1.", "1e"};
    for (unsigned i = 0; i < sizeof bad / sizeof bad[0]; i++) CHECK(json_parse(bad[i], strlen(bad[i]), t, 64) < 0);
    char deep[64];
    memset(deep, '[', 20);
    memset(deep + 20, ']', 20);
    CHECK(json_parse(deep, 40, t, 64) < 0);   // deeper than JSON_DEPTH
    CHECK(json_parse(s, strlen(s), t, 5) < 0);  // more tokens than room
}

TEST(helper_answers_are_read_safely) {
    size_t len;
    char *status = read_file("tests/fixtures/bluetooth/status.json", &len);
    CHECK(status != NULL);
    if (!status) return;
    BtReply r;
    CHECK(btapi_parse(status, len, &r) == 0 && r.ok && r.running && !r.scanning && r.ndev == 1);
    CHECK(!strcmp(r.dev[0].address, "02:00:5E:10:20:30") && !strcmp(r.dev[0].name, "Pro Controller") &&
          r.dev[0].classic && r.dev[0].connected);
    CHECK(!strcmp(r.version, "3.18.0-ea011ea"));
    free(status);
    const char *scan = "{\"ok\": true, \"scanning\": true, \"devices\": [{\"address\": \"aa:bb:cc:dd:ee:ff\", "
                       "\"name\": \"  8BitDo\\u0007\\u0008 Stick  \", \"protocol\": \"ble\"}, {\"address\": \"bad\"}, "
                       "{\"address\": \"AA:BB:CC:DD:EE:F\", \"name\": \"y\"}]}";
    CHECK(btapi_parse(scan, strlen(scan), &r) == 0 && r.ok && r.scanning && r.ndev == 1 && !r.dev[0].classic);
    CHECK(!strcmp(r.dev[0].name, "8BitDo? Stick"));  // radio names: printable ASCII, trimmed
    const char *err = "{\"ok\": false, \"error\": \"No scan in progress\"}";
    CHECK(btapi_parse(err, strlen(err), &r) == 0 && !r.ok && !strcmp(r.error, "No scan in progress"));
    CHECK(btapi_parse("[1]", 3, &r) < 0 && btapi_parse("{", 1, &r) < 0);
    CHECK(btapi_address_ok("AA:bb:01:23:45:67") && !btapi_address_ok("AA:bb:01:23:45:6") &&
          !btapi_address_ok("AA-bb-01-23-45-67") && !btapi_address_ok("AA:bb:01:23:45:6g"));
}

static BtEvent wait_event(BtReply *r) {
    for (int i = 0; i < 500; i++) {
        BtEvent e = bt_tick(r);
        if (e) return e;
        sleep_ms(10);
    }
    return BT_EV_NONE;
}

TEST(the_client_asks_one_question_at_a_time) {
    int port = 18000 + (int)(getpid() % 2000);
    char ps[16];
    snprintf(ps, sizeof ps, "%d", port);
    pid_t fake = fork();
    if (fake == 0) {
        execlp("python3", "python3", "-I", "tests/fake_bthelper.py", ps, (char *)NULL);
        _exit(127);
    }
    CHECK(fake > 0);
    char api[64];
    snprintf(api, sizeof api, "127.0.0.1:%d", port);
    bt_init(tl_tmp(), NULL, api);
    BtReply r;
    CHECK(bt_present());
    bt_start();
    CHECK(bt_state() == BT_EXTERNAL);  // a stand-in that runs already: Grimoire's to use, not to stop
    // (Wait for the stand-in to listen.)
    BtEvent e = BT_EV_FAILED;
    for (int i = 0; i < 50 && e == BT_EV_FAILED; i++) {
        CHECK(bt_request("/status"));
        e = wait_event(&r);
        if (e == BT_EV_FAILED) sleep_ms(100);
    }
    CHECK(e == BT_EV_REPLY && r.ok && r.ndev == 0);
    CHECK(bt_request("/scan") && !bt_request("/status") && bt_busy());  // one at a time
    CHECK(wait_event(&r) == BT_EV_REPLY && r.ok);
    sleep_ms(1100);
    CHECK(bt_request("/scan-status") && wait_event(&r) == BT_EV_REPLY && r.ndev == 2);
    CHECK(!strcmp(r.dev[1].name, "Stick? ?"));
    CHECK(bt_request("/pair?addr=02:00:5E:10:20:30&protocol=classic") && wait_event(&r) == BT_EV_REPLY);
    sleep_ms(900);
    CHECK(bt_request("/pair-status") && wait_event(&r) == BT_EV_REPLY && r.ok);
    CHECK(bt_request("/status") && wait_event(&r) == BT_EV_REPLY && r.ndev == 1 && r.dev[0].connected);
    CHECK(!bt_request("/a b") && !bt_request("status"));  // one plain line, a path
    kill(fake, SIGTERM);
    waitpid(fake, NULL, 0);
    CHECK(bt_request("/status") && wait_event(&r) == BT_EV_FAILED);  // gone: no answer
    bt_init(tl_tmp(), NULL, NULL);
    CHECK(!bt_present() && !bt_request("/status"));  // no script, no stand-in: no helper
}

int main(void) {
    RUN(json_reads_what_it_should_and_refuses_the_rest);
    RUN(helper_answers_are_read_safely);
    RUN(the_client_asks_one_question_at_a_time);
    return tl_done();
}
