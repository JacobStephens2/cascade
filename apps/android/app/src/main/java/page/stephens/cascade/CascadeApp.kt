package page.stephens.cascade

import android.app.Application
import page.stephens.cascade.core.CascadeBridgeHolder
import page.stephens.cascade.settings.SettingsStore

class CascadeApp : Application() {
    lateinit var settingsStore: SettingsStore
        private set
    lateinit var bridgeHolder: CascadeBridgeHolder
        private set

    override fun onCreate() {
        super.onCreate()
        settingsStore = SettingsStore(this)
        // Settings are read async; the bridge handles a missing/empty JSON
        // by booting with defaults.
        bridgeHolder = CascadeBridgeHolder(settingsStore)
    }
}
