package page.stephens.cascade.sync

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import page.stephens.cascade.core.Command
import page.stephens.cascade.core.Effect
import java.net.HttpURLConnection
import java.net.URL
import kotlin.coroutines.cancellation.CancellationException

/** Base URL of cascade-sync-server. Empty disables the account/sync feature. */
const val SYNC_API_BASE = "https://sync.cascade.stephens.page"

val syncAvailable: Boolean get() = SYNC_API_BASE.isNotEmpty()

/**
 * The one HTTP adapter for the core's sync-server requests, dependency-free
 * (HttpURLConnection on the IO dispatcher). It sends any
 * [Effect.ServerRequest] exactly as described and reports the status and body
 * verbatim; the core owns the request table and reads the response. Status 0
 * means no response: no sync server ([baseUrl] empty, nothing sent), a network
 * error or a timeout.
 */
class ServerHttp(
    private val baseUrl: String,
    private val timeoutMs: Int = 10_000,
) {
    /**
     * Send [request] in [scope] and hand its [Command.ServerResponse] to
     * [settle], exactly once — including when the request is cancelled
     * mid-flight, which settles with status 0. Until it is settled the core
     * holds the next account request or listening sync.
     */
    fun carry(
        request: Effect.ServerRequest,
        scope: CoroutineScope,
        settle: (Command.ServerResponse) -> Unit,
    ): Job = scope.launch {
        var response = unanswered(request)
        try {
            response = send(request)
        } finally {
            settle(response)
        }
    }

    private suspend fun send(request: Effect.ServerRequest): Command.ServerResponse {
        if (baseUrl.isEmpty()) return unanswered(request)
        return withContext(Dispatchers.IO) {
            try {
                exchange(request)
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                unanswered(request)
            }
        }
    }

    private fun exchange(request: Effect.ServerRequest): Command.ServerResponse {
        val conn = (URL(baseUrl + request.path).openConnection() as HttpURLConnection).apply {
            requestMethod = request.method
            connectTimeout = timeoutMs
            readTimeout = timeoutMs
            request.bearerToken?.let { setRequestProperty("Authorization", "Bearer $it") }
        }
        try {
            request.body?.let { body ->
                conn.setRequestProperty("Content-Type", "application/json")
                conn.doOutput = true
                conn.outputStream.use { it.write(body.toByteArray()) }
            }
            val status = conn.responseCode
            val stream = if (status in 200..299) conn.inputStream else conn.errorStream
            val body = stream?.bufferedReader()?.use { it.readText() }.orEmpty()
            return Command.ServerResponse(request.id, status, body)
        } finally {
            conn.disconnect()
        }
    }

    private fun unanswered(request: Effect.ServerRequest) =
        Command.ServerResponse(request.id, status = 0, body = "")
}
