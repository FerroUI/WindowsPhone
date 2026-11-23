package org.ferroui.phone;

import android.Manifest;
import android.content.ComponentName;
import android.content.Context;
import android.os.Build;
import android.telecom.PhoneAccount;
import android.telecom.PhoneAccountHandle;
import android.telecom.TelecomManager;
import android.telephony.SubscriptionInfo;
import android.telephony.SubscriptionManager;
import android.telephony.SmsManager;
import android.util.Base64;
import java.nio.charset.StandardCharsets;
import java.util.Collections;
import java.util.List;
import org.json.JSONArray;
import org.json.JSONObject;

/** Telecom account handles and SMS subscriptions deliberately use different identity spaces. */
final class PhoneAccounts {
    private PhoneAccounts() {}

    static String id(PhoneAccountHandle handle) {
        return id(handle.getComponentName().flattenToString(), handle.getId());
    }

    static String id(String component, String value) {
        if (component == null || value == null || component.isEmpty() || value.isEmpty()) return null;
        String id = "phone-account:" + Base64.encodeToString((component + "\n" + value).getBytes(StandardCharsets.UTF_8), Base64.URL_SAFE | Base64.NO_WRAP | Base64.NO_PADDING);
        return id.length() <= 1024 ? id : null;
    }

    static JSONArray calls(Context context) {
        JSONArray rows = new JSONArray();
        if (!PhonePermissions.granted(context, Manifest.permission.READ_PHONE_STATE)) return rows;
        TelecomManager manager = context.getSystemService(TelecomManager.class);
        if (manager == null) return rows;
        PhoneAccountHandle selected = manager.getDefaultOutgoingPhoneAccount(PhoneAccount.SCHEME_TEL);
        for (PhoneAccountHandle handle : manager.getCallCapablePhoneAccounts()) {
            if (rows.length() >= 16) break;
            PhoneAccount account = manager.getPhoneAccount(handle);
            String id = id(handle);
            if (id == null || account == null || !account.isEnabled()) continue;
            String label = PhoneJson.text(account.getLabel(), 512, false).trim();
            if (label.isEmpty()) label = "Phone account";
            JSONObject row = PhoneJson.put(new JSONObject(), "id", id);
            PhoneJson.put(row, "label", label);
            PhoneJson.put(row, "is_default", handle.equals(selected));
            rows.put(row);
        }
        return rows;
    }

    static PhoneAccountHandle resolveCall(Context context, String id) {
        if (!PhonePermissions.granted(context, Manifest.permission.READ_PHONE_STATE)) return null;
        TelecomManager manager = context.getSystemService(TelecomManager.class);
        if (manager == null) return null;
        for (PhoneAccountHandle handle : manager.getCallCapablePhoneAccounts()) {
            PhoneAccount account = manager.getPhoneAccount(handle);
            if (id.equals(id(handle)) && account != null && account.isEnabled()) return handle;
        }
        return null;
    }

    private static List<SubscriptionInfo> subscriptions(Context context) {
        if (!PhonePermissions.granted(context, Manifest.permission.READ_PHONE_STATE)) return Collections.emptyList();
        SubscriptionManager manager = context.getSystemService(SubscriptionManager.class);
        List<SubscriptionInfo> subscriptions = manager == null ? null : manager.getActiveSubscriptionInfoList();
        return subscriptions == null ? Collections.emptyList() : subscriptions;
    }

    static JSONArray messages(Context context) {
        JSONArray rows = new JSONArray();
        int selected = SubscriptionManager.getDefaultSmsSubscriptionId();
        for (SubscriptionInfo subscription : subscriptions(context)) {
            if (rows.length() >= 16) break;
            String label = PhoneJson.text(subscription.getDisplayName(), 512, false).trim();
            if (label.isEmpty()) label = "SIM " + (subscription.getSimSlotIndex() + 1);
            JSONObject row = PhoneJson.put(new JSONObject(), "id", "sms-subscription:" + subscription.getSubscriptionId());
            PhoneJson.put(row, "label", label);
            PhoneJson.put(row, "is_default", subscription.getSubscriptionId() == selected);
            rows.put(row);
        }
        return rows;
    }

    static int resolveSms(Context context, String requested) {
        int selected = SubscriptionManager.getDefaultSmsSubscriptionId();
        if (requested != null) {
            selected = -1;
            for (SubscriptionInfo subscription : subscriptions(context)) {
                if (requested.equals("sms-subscription:" + subscription.getSubscriptionId())) {
                    selected = subscription.getSubscriptionId(); break;
                }
            }
        }
        // An unset/invalid default on a multi-SIM device must never choose an arbitrary SIM.
        if (!SubscriptionManager.isValidSubscriptionId(selected))
            throw new IllegalArgumentException("Choose an active SMS subscription or select Android's default SMS SIM first.");
        return selected;
    }

    static SmsManager sms(Context context, int subscription) {
        if (!SubscriptionManager.isValidSubscriptionId(subscription)) throw new IllegalArgumentException("Invalid SMS subscription.");
        return Build.VERSION.SDK_INT >= 31 ? context.getSystemService(SmsManager.class).createForSubscriptionId(subscription)
                : SmsManager.getSmsManagerForSubscriptionId(subscription);
    }
}
