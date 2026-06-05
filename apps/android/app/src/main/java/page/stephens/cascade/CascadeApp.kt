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
        bridgeHolder = CascadeBridgeHolder(settingsStore)
        syncManager = SyncManager(bridgeHolder, AccountStore(this))
    }
}
