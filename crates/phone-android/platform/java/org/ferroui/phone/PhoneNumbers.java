package org.ferroui.phone;

/** Matches the shared numeric dial/SMS-address contract without interpreting schemes or query parameters. */
final class PhoneNumbers {
    private PhoneNumbers() {}
    static boolean valid(String value) {
        if (value == null || value.length() > 128 || value.trim().isEmpty()) return false;
        boolean digit = false;
        for (int i = 0; i < value.length(); i++) {
            char c = value.charAt(i);
            if (c >= '0' && c <= '9' || c == '*' || c == '#') digit = true;
            else if ("+ -().,;".indexOf(c) < 0) return false;
        }
        return digit;
    }
    static String messageAddress(String value) {
        if (!valid(value) || value.indexOf(',') >= 0 || value.indexOf(';') >= 0 || value.indexOf('*') >= 0 || value.indexOf('#') >= 0) return null;
        String normalized = value.replace(" ", "").replace("-", "").replace("(", "").replace(")", "").replace(".", "");
        return normalized.matches("\\+?[0-9]{1,30}") ? normalized : null;
    }
}
