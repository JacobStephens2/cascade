package page.stephens.cascade.sync

import android.content.Context
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.stringPreferencesKey
import androidx.datastore.preferences.preferencesDataStore
import kotlinx.coroutines.flow.first

private val Context.accountDataStore by preferencesDataStore("cascade-account")

data class Account(val sessionToken: String, val email: String)

/**
 * Persists the optional sync account (session token + email). The per-device
 * id (this device's G-Counter slot) now lives in the core's listening blob;
 * this store only exposes the id older builds kept here, so the core can adopt
 * it once and an existing server slot carries over.
 */
class AccountStore(private val context: Context) {
    private val tokenKey = stringPreferencesKey("session_token")
    private val emailKey = stringPreferencesKey("email")
    private val deviceKey = stringPreferencesKey("device_id")

    suspend fun readAccount(): Account? {
        val prefs = context.accountDataStore.data.first()
        val token = prefs[tokenKey]
        val email = prefs[emailKey]
        return if (token != null && email != null) Account(token, email) else null
    }

    suspend fun writeAccount(account: Account) {
        context.accountDataStore.edit {
            it[tokenKey] = account.sessionToken
            it[emailKey] = account.email
        }
    }

    suspend fun clearAccount() {
        context.accountDataStore.edit {
            it.remove(tokenKey)
            it.remove(emailKey)
        }
    }

    /**
     * The device id older builds generated and stored here, or null. Read-only:
     * handed to the core as the restore fallback; the core owns (and rotates)
     * the id from then on.
     */
    suspend fun legacyDeviceId(): String? = context.accountDataStore.data.first()[deviceKey]
}
