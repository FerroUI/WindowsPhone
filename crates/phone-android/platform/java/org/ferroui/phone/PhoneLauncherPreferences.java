package org.ferroui.phone;

import android.app.Activity;
import android.graphics.Color;
import android.os.Bundle;
import android.widget.Button;
import android.widget.CheckBox;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.TextView;

/** Android app/Quick Settings preferences; changes require a visible user action. */
public final class PhoneLauncherPreferences extends Activity {
    @Override protected void onCreate(Bundle saved) { super.onCreate(saved); render(); }
    @Override protected void onResume() { super.onResume(); render(); }
    private void render() {
        LinearLayout content = new LinearLayout(this); content.setOrientation(LinearLayout.VERTICAL);
        int padding = Math.round(24 * getResources().getDisplayMetrics().density); content.setPadding(padding, padding, padding, padding);
        ScrollView scroll = new ScrollView(this); scroll.setBackgroundColor(Color.BLACK); scroll.addView(content); setContentView(scroll);
        TextView title = new TextView(this); title.setText("launcher appearance"); title.setTextSize(30); title.setTextColor(Color.WHITE); content.addView(title);
        boolean held = PhoneLauncher.held(this);
        CheckBox immersive = new CheckBox(this); immersive.setText("Hide Android bars while Windows Phone is Home"); immersive.setTextColor(Color.WHITE);
        immersive.setChecked(getSharedPreferences("phone-launcher", MODE_PRIVATE).getBoolean("immersive", false)); immersive.setEnabled(held);
        immersive.setOnCheckedChangeListener((view, checked) -> {
            if (PhoneLauncher.held(this)) getSharedPreferences("phone-launcher", MODE_PRIVATE).edit().putBoolean("immersive", checked).apply();
        }); content.addView(immersive);
        TextView explanation = new TextView(this); explanation.setText("Swipe from an edge to reveal Android's system bars. Android gestures and the lock screen remain available. The setting applies only while this package is your selected Home app.");
        explanation.setTextColor(0xffcccccc); explanation.setTextSize(17); content.addView(explanation);
        if (!held && PhoneLauncher.available(this)) {
            Button role = new Button(this); role.setText("Choose Home app"); role.setOnClickListener(view -> PhoneLauncher.request(this)); content.addView(role);
        }
        scroll.setOnApplyWindowInsetsListener((view, insets) -> {
            view.setPadding(insets.getSystemWindowInsetLeft(), insets.getSystemWindowInsetTop(), insets.getSystemWindowInsetRight(), insets.getSystemWindowInsetBottom()); return insets;
        });
    }
}
