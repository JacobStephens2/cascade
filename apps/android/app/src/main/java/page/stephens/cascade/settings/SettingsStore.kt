package page.stephens.cascade.settings

import android.content.Context
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.stringPreferencesKey
import androidx.datastore.preferences.preferencesDataStore
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.map

private val Context.dataStore by preferencesDataStore("cascade-settings")

/**
 * Stores the cascade-core settings blob (JSON produced by the Rust
 * `PersistSettings` effect). The JSON shape is opaque to Kotlin — we just
 * round-trip it.
 */
class SettingsStore(private val context: Context) {
    private val key = stringPreferencesKey("settings_v1")

    suspend fun read(): String? =
        context.dataStore.data.map { it[key] }.first()

    suspend fun write(json: String) {
        context.dataStore.edit { it[key] = json }
    }
}
