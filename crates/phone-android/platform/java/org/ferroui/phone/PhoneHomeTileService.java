package org.ferroui.phone;

import android.app.PendingIntent;
import android.content.Intent;
import android.net.Uri;
import android.os.Build;
import android.service.quicksettings.Tile;
import android.service.quicksettings.TileService;

/** Optional Android Quick Settings entry. Adding a tile and selecting HOME remain Android user actions. */
public final class PhoneHomeTileService extends TileService {
    @Override public void onStartListening() {
        super.onStartListening(); Tile tile = getQsTile();
        if (tile != null) { tile.setLabel("Windows Phone"); tile.setState(PhoneLauncher.held(this) ? Tile.STATE_ACTIVE : Tile.STATE_INACTIVE); tile.updateTile(); }
    }
    @Override public void onClick() {
        super.onClick();
        Intent open = new Intent(this, PhoneActivity.class).setAction(Intent.ACTION_VIEW).setData(Uri.parse("ferroui-phone://home"))
                .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK | Intent.FLAG_ACTIVITY_CLEAR_TOP | Intent.FLAG_ACTIVITY_SINGLE_TOP);
        if (Build.VERSION.SDK_INT >= 34) startActivityAndCollapse(PendingIntent.getActivity(this, 0, open, PendingIntent.FLAG_UPDATE_CURRENT | PendingIntent.FLAG_IMMUTABLE));
        else startActivityAndCollapse(open);
    }
}
