package page.stephens.cascade.sync

import android.content.Context
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.stringPreferencesKey
import androidx.datastore.preferences.preferencesDataStore
import kotlinx.coroutines.flow.first
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json

private val Context.accountDataStore by preferencesDataStore("cascade-account")

/** The version-less shape this shell stored before the core held the account. */
@Serializable
private data class LegacyAccount(val sessionToken: String, val email: String)

/**
 * Stores the core's account blob (JSON produced by the Rust `PersistAccount`
 * effect), opaque to Kotlin. Before the core held the account this store kept
 * the session token and email under their own keys; on the first launch after
 * upgrading those are converted once to the version-less
 * `{ sessionToken, email }` the core also reads, so an existing sign-in
 * survives.
 *
 * The per-device id (this device's G-Counter slot) lives in the core's
 * listening blob; this store only exposes the id older builds kept here, so
 * the core can adopt it once and an existing server slot carries over.
 */
class AccountStore(private val context: Context) {
    private val accountKey = stringPreferencesKey("account_v1")
    private val legacyTokenKey = stringPreferencesKey("session_token")
    private val legacyEmailKey = stringPreferencesKey("email")
    private val deviceKey = stringPreferencesKey("device_id")

    /** The stored blob, or `""`. A legacy account is converted and stored once. */
    suspend fun read(): String {
        val prefs = context.accountDataStore.data.first()
        prefs[accountKey]?.let { return it }
        val token = prefs[legacyTokenKey] ?: return ""
        val email = prefs[legacyEmailKey] ?: return ""
        val converted = Json.encodeToString(LegacyAccount.serializer(), LegacyAccount(token, email))
        write(converted)
        return converted
    }

    /** Store the blob verbatim; an empty [json] deletes the stored account. */
    suspend fun write(json: String) {
        context.accountDataStore.edit {
            if (json.isEmpty()) it.remove(accountKey) else it[accountKey] = json
            it.remove(legacyTokenKey)
            it.remove(legacyEmailKey)
        }
    }

    /**
     * The device id older builds generated and stored here, or null. Read-only:
     * handed to the core as the restore fallback; the core owns (and rotates)
     * the id from then on.
     */
    suspend fun legacyDeviceId(): String? = context.accountDataStore.data.first()[deviceKey]
}
