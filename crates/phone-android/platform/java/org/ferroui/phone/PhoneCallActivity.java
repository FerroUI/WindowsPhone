package org.ferroui.phone;

import android.app.Activity;
import android.content.Intent;
import android.graphics.Color;
import android.os.Build;
import android.os.Bundle;
import android.view.Gravity;
import android.view.View;
import android.view.WindowInsets;
import android.view.WindowManager;
import android.widget.Button;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.TextView;
import android.widget.Toast;
import java.lang.ref.WeakReference;
import org.json.JSONArray;
import org.json.JSONObject;

/** A complete native call surface for lock-screen/background entry, independent of Rust host lifetime. */
public final class PhoneCallActivity extends Activity {
    private static WeakReference<PhoneCallActivity> current = new WeakReference<>(null);
    private LinearLayout content;
    private String last = "";
    private boolean visible;

    static boolean isVisible() { PhoneCallActivity activity = current.get(); return activity != null && activity.visible; }

    static void changed() {
        PhoneCallActivity activity = current.get();
        if (activity != null && activity.visible) activity.render();
    }

    @Override protected void onCreate(Bundle saved) {
        super.onCreate(saved);
        current = new WeakReference<>(this);
        if (Build.VERSION.SDK_INT >= 27) { setShowWhenLocked(true); setTurnScreenOn(true); }
        else getWindow().addFlags(WindowManager.LayoutParams.FLAG_SHOW_WHEN_LOCKED | WindowManager.LayoutParams.FLAG_TURN_SCREEN_ON);
        getWindow().addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);
        ScrollView scroll = new ScrollView(this);
        scroll.setBackgroundColor(Color.BLACK);
        content = new LinearLayout(this);
        content.setOrientation(LinearLayout.VERTICAL);
        content.setPadding(dp(20), dp(20), dp(20), dp(20));
        scroll.addView(content);
        scroll.setOnApplyWindowInsetsListener((view, insets) -> {
            int left, top, right, bottom;
            if (Build.VERSION.SDK_INT >= 30) {
                android.graphics.Insets edges = insets.getInsets(WindowInsets.Type.systemBars() | WindowInsets.Type.displayCutout());
                left = edges.left; top = edges.top; right = edges.right; bottom = edges.bottom;
            } else { left = insets.getSystemWindowInsetLeft(); top = insets.getSystemWindowInsetTop(); right = insets.getSystemWindowInsetRight(); bottom = insets.getSystemWindowInsetBottom(); }
            view.setPadding(left, top, right, bottom);
            return insets;
        });
        setContentView(scroll);
        apply(getIntent());
    }

    @Override protected void onNewIntent(Intent intent) { super.onNewIntent(intent); setIntent(intent); apply(intent); }
    @Override protected void onResume() { super.onResume(); visible = true; current = new WeakReference<>(this); render(); }
    @Override protected void onPause() { visible = false; super.onPause(); }
    @Override protected void onDestroy() { if (current.get() == this) current.clear(); super.onDestroy(); }

    private void apply(Intent intent) {
        String id = intent.getStringExtra("call"), command = intent.getStringExtra("command");
        if (id != null && id.length() <= 128 && "answer".equals(command)) {
            intent.removeExtra("command");
            result(PhoneInCallService.action(id, PhoneJson.value("answer")));
        }
    }

    private void render() {
        JSONArray calls = PhoneInCallService.snapshot(this);
        String signature = calls.toString();
        if (signature.equals(last)) return;
        last = signature;
        content.removeAllViews();
        text("PHONE", 16, Color.WHITE);
        if (calls.length() == 0) {
            text("call ended", 36, Color.WHITE);
            button("Return to phone", () -> { Intent intent = PhoneActivity.openIntent(this); if (intent != null) startActivity(intent); finish(); });
            return;
        }
        for (int index = 0; index < calls.length(); index++) {
            JSONObject call = calls.optJSONObject(index);
            if (call == null) continue;
            String id = call.optString("id"), name = call.optString("name"), number = call.optString("number"), state = call.optString("state");
            text(name.isEmpty() ? number.isEmpty() ? "unknown caller" : number : name, 32, Color.WHITE);
            if (!name.isEmpty() && !number.isEmpty()) text(number, 20, 0xffb5b5b5);
            text(state, 18, 0xff00a5df);
            PhoneInCallService service = PhoneInCallService.current();
            JSONArray accounts = service == null ? new JSONArray() : service.pendingAccounts(id);
            for (int item = 0; item < accounts.length(); item++) {
                JSONObject account = accounts.optJSONObject(item);
                if (account == null) continue;
                button("Use " + account.optString("label"), () -> result(service.selectAccount(id, account.optString("id"))));
            }
            if ("ringing".equals(state)) {
                button("answer", () -> result(PhoneInCallService.action(id, PhoneJson.value("answer"))));
                button("decline", () -> result(PhoneInCallService.action(id, PhoneJson.value("reject"))));
            } else if (!"disconnected".equals(state)) {
                button("end call", () -> result(PhoneInCallService.action(id, PhoneJson.value("end"))));
                button(call.optBoolean("muted") ? "unmute" : "mute", () -> result(PhoneInCallService.action(id,
                        PhoneJson.put(PhoneJson.value("setMuted"), "muted", !call.optBoolean("muted")))));
                button(call.optBoolean("speaker") ? "phone audio" : "speaker", () -> result(PhoneInCallService.action(id,
                        PhoneJson.put(PhoneJson.value("setSpeaker"), "enabled", !call.optBoolean("speaker")))));
                if (call.optBoolean("can_hold") || "holding".equals(state))
                    button("holding".equals(state) ? "resume" : "hold", () -> result(PhoneInCallService.action(id,
                            PhoneJson.value("holding".equals(state) ? "resume" : "hold"))));
                if ("active".equals(state)) dialpad(id);
            }
        }
    }

    private void dialpad(String id) {
        String[] rows = {"123", "456", "789", "*0#"};
        for (String row : rows) {
            LinearLayout line = new LinearLayout(this);
            for (int i = 0; i < row.length(); i++) {
                String tone = row.substring(i, i + 1);
                Button button = new Button(this);
                button.setText(tone);
                button.setTextSize(24);
                button.setMinHeight(dp(56));
                button.setOnClickListener(view -> result(PhoneInCallService.action(id, PhoneJson.put(PhoneJson.value("dtmf"), "tone", tone))));
                line.addView(button, new LinearLayout.LayoutParams(0, dp(56), 1));
            }
            content.addView(line);
        }
    }

    private void text(String text, int size, int color) {
        TextView view = new TextView(this);
        view.setText(text); view.setTextSize(size); view.setTextColor(color);
        view.setPadding(0, dp(8), 0, dp(12));
        content.addView(view);
    }
    private void button(String label, Runnable click) {
        Button button = new Button(this);
        button.setText(label); button.setAllCaps(false); button.setTextSize(20); button.setMinHeight(dp(52));
        button.setOnClickListener(view -> click.run());
        content.addView(button, new LinearLayout.LayoutParams(-1, -2));
    }
    private void result(JSONObject result) {
        if ("error".equals(result.optString("kind"))) Toast.makeText(this, result.optString("message"), Toast.LENGTH_LONG).show();
    }
    private int dp(int value) { return Math.round(value * getResources().getDisplayMetrics().density); }
}
