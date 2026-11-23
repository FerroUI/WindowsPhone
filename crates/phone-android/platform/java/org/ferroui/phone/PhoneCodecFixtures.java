package org.ferroui.phone;

import android.content.Context;
import com.android.messaging.mmslib.pdu.*;
import java.io.ByteArrayInputStream;
import java.io.ByteArrayOutputStream;
import java.nio.charset.StandardCharsets;
import java.util.Arrays;
import org.json.JSONArray;
import org.json.JSONObject;

/** Local codec fixtures only; no provider writes, RF request, permission request or activation. */
final class PhoneCodecFixtures {
    private PhoneCodecFixtures() {}
    private static void check(boolean condition, String name) { if (!condition) throw new IllegalStateException("MMS codec fixture failed: " + name); }
    private static final class Probe extends PduParser {
        Probe() { super(new byte[]{0}, true); }
        PduBody parts(byte[] bytes) { return parseParts(new ByteArrayInputStream(bytes)); }
    }
    static JSONObject run(Context context) throws Exception {
        JSONObject request = PhoneJson.put(new JSONObject(), "recipients", new JSONArray().put("5550101").put("5550102"));
        PhoneJson.put(request, "subject", "Codec round trip"); PhoneJson.put(request, "body", "A real UTF-8 PDU: 😀\nsecond line"); PhoneJson.put(request, "attachments", new JSONArray());
        SendReq sent = PhoneMmsCodec.request(context, request);
        PduPart binary = new PduPart(); binary.setContentType("application/octet-stream".getBytes(StandardCharsets.US_ASCII));
        binary.setContentLocation("fixture.bin".getBytes(StandardCharsets.US_ASCII)); binary.setData(new byte[]{0, 1, 2, (byte)255}); sent.getBody().addPart(binary);
        byte[] encoded = PhoneMmsCodec.compose(context, sent); GenericPdu parsed = PhoneMmsCodec.parse(encoded);
        check(parsed instanceof SendReq, "SendReq type"); SendReq decoded = (SendReq) parsed;
        check(decoded.getTo().length == 2, "group recipient preservation");
        check("Codec round trip".equals(decoded.getSubject().getString()), "subject preservation");
        check(decoded.getBody().getPartsNum() == 2, "multipart preservation");
        check(request.optString("body").equals(new String(decoded.getBody().getPart(0).getData(), StandardCharsets.UTF_8)), "Unicode body");
        check(Arrays.equals(binary.getData(), decoded.getBody().getPart(1).getData()), "binary attachment");
        check(PhoneMmsCodec.parse(Arrays.copyOf(encoded, encoded.length - 1)) == null, "truncated body rejection");
        Probe probe = new Probe();
        check(probe.parts(new byte[]{101}) == null, "part-count bound");
        check(probe.parts(new byte[]{1, 127, 0}) == null, "truncated header bound");
        check(probe.parts(new byte[]{1, 1, 127, (byte)0x83}) == null, "untrusted data-length bound");
        byte[] body = part("text/plain", "ok".getBytes(StandardCharsets.UTF_8));
        check(new Probe().parts(body) != null, "independent valid part fixture");
        for (int index = 0; index < 5; index++) body = part("application/vnd.wap.multipart.alternative", body);
        check(new Probe().parts(body) == null, "nested multipart depth bound");
        NotifyRespInd acknowledgement = new NotifyRespInd(PduHeaders.CURRENT_MMS_VERSION, "fixture-transaction".getBytes(StandardCharsets.US_ASCII), PduHeaders.STATUS_RETRIEVED);
        check(PhoneMmsCodec.parse(PhoneMmsCodec.compose(context, acknowledgement)) instanceof NotifyRespInd, "notification response encoding");
        JSONObject result = PhoneJson.put(new JSONObject(), "result", "passed");
        PhoneJson.put(result, "assertions", 13); PhoneJson.put(result, "encoded_bytes", encoded.length);
        PhoneJson.put(result, "rf_requests", 0); return result;
    }
    private static byte[] part(String mime, byte[] data) {
        ByteArrayOutputStream out = new ByteArrayOutputStream(); byte[] header = mime.getBytes(StandardCharsets.US_ASCII);
        out.write(1); uint(out, header.length + 1); uint(out, data.length); out.write(header, 0, header.length); out.write(0); out.write(data, 0, data.length); return out.toByteArray();
    }
    private static void uint(ByteArrayOutputStream out, int value) {
        int groups = 1; for (int remaining = value >>> 7; remaining != 0; remaining >>>= 7) groups++;
        for (int index = groups - 1; index >= 0; index--) out.write((value >>> (index * 7) & 127) | (index == 0 ? 0 : 128));
    }
}
