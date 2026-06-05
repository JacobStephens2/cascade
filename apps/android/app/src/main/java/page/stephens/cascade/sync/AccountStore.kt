package page.stephens.cascade.sync

import android.content.Context
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.stringPreferencesKey
import androidx.datastore.preferences.preferencesDataStore
import kotlinx.coroutines.flow.first
import java.util.UUID

private val Context.accountDataStore by preferencesDataStore("cascade-account")

data class Account(val sessionToken: String, val email: String)

/**
 * Persists the optional sync account (session token + email) and the stable
 * per-device id used as this device's G-Counter slot. The device id is rotated
 * on "delete data" so a stale offline write can't resurrect a deleted total.
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

    /** Stable device id, created on first use. */
    suspend fun deviceId(): String {
        val existing = context.accountDataStore.data.first()[deviceKey]
        if (existing != null) return existing
        val id = UUID.randomUUID().toString()
        context.accountDataStore.edit { it[deviceKey] = id }
        return id
    }

    /** New device id; returns it. Call when deleting listening data. */
    suspend fun rotateDeviceId(): String {
        val id = UUID.randomUUID().toString()
        context.accountDataStore.edit { it[deviceKey] = id }
        return id
    }
}
