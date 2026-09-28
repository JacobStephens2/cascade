package page.stephens.cascade

import android.app.Application
import page.stephens.cascade.core.CascadeBridgeHolder
import page.stephens.cascade.settings.SettingsStore
import page.stephens.cascade.sync.AccountStore
import page.stephens.cascade.sync.SyncManager

class CascadeApp : Application() {
    lateinit var settingsStore: SettingsStore
        private set
    lateinit var bridgeHolder: CascadeBridgeHolder
        private set
    lateinit var syncManager: SyncManager
        private set

    override fun onCreate() {
        super.onCreate()
        settingsStore = SettingsStore(this)
        // Settings are read async; the bridge handles a missing/empty JSON
        // by booting with defaults.
        val accountStore = AccountStore(this)
        // The legacy device id is read once, at restore, so an existing
        // server slot carries over to the core-owned id.
        bridgeHolder = CascadeBridgeHolder(settingsStore) { accountStore.legacyDeviceId() }
        syncManager = SyncManager(bridgeHolder, accountStore)
    }
}
