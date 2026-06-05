package page.stephens.cascade.sync

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import java.io.BufferedReader
import java.net.HttpURLConnection
import java.net.URL

/** Base URL of cascade-sync-server. Empty disables the account/sync feature. */
const val SYNC_API_BASE = "https://sync.cascade.stephens.page"

val syncAvailable: Boolean get() = SYNC_API_BASE.isNotEmpty()

class SyncException(val status: Int, message: String) : Exception(message)

@Serializable
data class VerifyResponse(val sessionToken: String, val email: String)

@Serializable
data class ListeningResponse(val serverTotalMs: Long, val syncedThroughMs: Long = 0)

@Serializable
private data class EmailBody(val email: String)

@Serializable
private data class TokenBody(val token: String)

@Serializable
private data class ListeningBody(val deviceId: String, val deviceTotalMs: Long)

/**
 * Thin HTTP client for the sync service, dependency-free (HttpURLConnection on
 * the IO dispatcher). Throws [SyncException] with the status on non-2xx.
 */
object SyncApi {
    private val json = Json { ignoreUnknownKeys = true }

    suspend fun requestLink(email: String) {
        post("/auth/request", json.encodeToString(EmailBody.serializer(), EmailBody(email)), null)
    }

    suspend fun verify(token: String): VerifyResponse {
        val body = post("/auth/verify", json.encodeToString(TokenBody.serializer(), TokenBody(token)), null)
        return json.decodeFromString(VerifyResponse.serializer(), body)
    }

    suspend fun logout(sessionToken: String) {
        post("/auth/logout", "", sessionToken)
    }

    suspend fun putListening(
        sessionToken: String,
        deviceId: String,
        deviceTotalMs: Long,
    ): ListeningResponse {
        val payload = json.encodeToString(
            ListeningBody.serializer(),
            ListeningBody(deviceId, deviceTotalMs),
        )
        val body = request("PUT", "/listening", payload, sessionToken)
        return json.decodeFromString(ListeningResponse.serializer(), body)
    }

    suspend fun deleteListening(sessionToken: String) {
        request("DELETE", "/listening", null, sessionToken)
    }

    suspend fun deleteAccount(sessionToken: String) {
        request("DELETE", "/account", null, sessionToken)
    }

    private suspend fun post(path: String, body: String, token: String?): String =
        request("POST", path, body, token)

    private suspend fun request(
        method: String,
        path: String,
        body: String?,
        token: String?,
    ): String = withContext(Dispatchers.IO) {
        val conn = (URL("$SYNC_API_BASE$path").openConnection() as HttpURLConnection).apply {
            requestMethod = method
            connectTimeout = 10_000
            readTimeout = 10_000
            setRequestProperty("Content-Type", "application/json")
            token?.let { setRequestProperty("Authorization", "Bearer $it") }
            if (body != null) {
                doOutput = true
                outputStream.use { it.write(body.toByteArray()) }
            }
        }
        try {
            val code = conn.responseCode
            if (code in 200..299) {
                conn.inputStream.bufferedReader().use(BufferedReader::readText)
            } else {
                val err = conn.errorStream?.bufferedReader()?.use(BufferedReader::readText).orEmpty()
                throw SyncException(code, err.ifEmpty { "request failed ($code)" })
            }
        } finally {
            conn.disconnect()
        }
    }
}
