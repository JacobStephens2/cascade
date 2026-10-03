package page.stephens.cascade.sync

import com.sun.net.httpserver.HttpExchange
import com.sun.net.httpserver.HttpServer
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.delay
import kotlinx.coroutines.runBlocking
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import page.stephens.cascade.core.Command
import page.stephens.cascade.core.Effect
import java.net.InetSocketAddress
import java.net.ServerSocket
import java.util.concurrent.CountDownLatch
import java.util.concurrent.LinkedBlockingQueue
import java.util.concurrent.TimeUnit

class ServerHttpTest {
    /** What the fake sync server saw of one request. */
    private data class Seen(
        val method: String,
        val path: String,
        val authorization: String?,
        val contentType: String?,
        val body: String,
    )

    private val seen = LinkedBlockingQueue<Seen>()
    private var server: HttpServer? = null
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)

    @After
    fun tearDown() {
        scope.cancel()
        server?.stop(0)
    }

    /** A local sync server that records each request and answers with [answer]. */
    private fun serve(answer: (HttpExchange) -> Pair<Int, String>): String {
        val http = HttpServer.create(InetSocketAddress("127.0.0.1", 0), 0)
        http.createContext("/") { exchange ->
            seen.add(
                Seen(
                    method = exchange.requestMethod,
                    path = exchange.requestURI.path,
                    authorization = exchange.requestHeaders.getFirst("Authorization"),
                    contentType = exchange.requestHeaders.getFirst("Content-Type"),
                    body = exchange.requestBody.bufferedReader().use { it.readText() },
                ),
            )
            val (status, body) = answer(exchange)
            val bytes = body.toByteArray()
            exchange.sendResponseHeaders(status, if (bytes.isEmpty()) -1 else bytes.size.toLong())
            if (bytes.isNotEmpty()) exchange.responseBody.use { it.write(bytes) }
            exchange.close()
        }
        http.start()
        server = http
        return "http://127.0.0.1:${http.address.port}"
    }

    private fun request(
        id: Long = 7,
        method: String = "POST",
        path: String = "/auth/request",
        bearerToken: String? = null,
        body: String? = null,
    ) = Effect.ServerRequest(id, method, path, bearerToken, body)

    /** Carry [request] and wait for its one settle. */
    private fun settle(http: ServerHttp, request: Effect.ServerRequest): Command.ServerResponse {
        val settles = LinkedBlockingQueue<Command.ServerResponse>()
        runBlocking { http.carry(request, scope) { settles.add(it) }.join() }
        assertEquals("settled exactly once", 1, settles.size)
        return settles.single()
    }

    @Test
    fun sendsTheRequestVerbatimAndSettlesWithStatusAndBody() {
        val base = serve { 200 to """{"serverTotalMs":42}""" }
        val response = settle(
            ServerHttp(base),
            request(
                id = 11,
                method = "PUT",
                path = "/listening",
                bearerToken = "tok",
                body = """{"deviceId":"d","deviceTotalMs":5}""",
            ),
        )

        assertEquals(Command.ServerResponse(11, 200, """{"serverTotalMs":42}"""), response)
        val sent = seen.single()
        assertEquals("PUT", sent.method)
        assertEquals("/listening", sent.path)
        assertEquals("Bearer tok", sent.authorization)
        assertEquals("application/json", sent.contentType)
        assertEquals("""{"deviceId":"d","deviceTotalMs":5}""", sent.body)
    }

    @Test
    fun aRequestWithNoBearerOrBodySendsNeither() {
        val base = serve { 204 to "" }
        val response = settle(ServerHttp(base), request(method = "DELETE", path = "/account"))

        assertEquals(Command.ServerResponse(7, 204, ""), response)
        val sent = seen.single()
        assertEquals("DELETE", sent.method)
        assertNull(sent.authorization)
        assertEquals("", sent.body)
    }

    @Test
    fun aBodilessPostIsSent() {
        // The sign-out revoke: POST /auth/logout with only the bearer token.
        val base = serve { 204 to "" }
        val response = settle(ServerHttp(base), request(path = "/auth/logout", bearerToken = "tok"))

        assertEquals(204, response.status)
        assertEquals("POST", seen.single().method)
        assertEquals("Bearer tok", seen.single().authorization)
    }

    @Test
    fun anErrorStatusIsSettledWithItsBody() {
        val base = serve { 401 to "unauthorized" }
        val response = settle(ServerHttp(base), request(bearerToken = "stale"))

        assertEquals(Command.ServerResponse(7, 401, "unauthorized"), response)
    }

    @Test
    fun anErrorStatusWithNoBodySettlesWithAnEmptyBody() {
        val base = serve { 500 to "" }
        assertEquals(Command.ServerResponse(7, 500, ""), settle(ServerHttp(base), request()))
    }

    @Test
    fun noSyncServerSettlesWithStatusZeroAndSendsNothing() {
        serve { 200 to "" }
        val response = settle(ServerHttp(""), request(body = "{}"))

        assertEquals(Command.ServerResponse(7, 0, ""), response)
        assertTrue(seen.isEmpty())
    }

    @Test
    fun aNetworkErrorSettlesWithStatusZero() {
        val closedPort = ServerSocket(0).use { it.localPort }
        val response = settle(ServerHttp("http://127.0.0.1:$closedPort"), request(body = "{}"))

        assertEquals(Command.ServerResponse(7, 0, ""), response)
    }

    @Test
    fun aTimeoutSettlesWithStatusZero() {
        val base = serve { Thread.sleep(1_000); 200 to "late" }
        val response = settle(ServerHttp(base, timeoutMs = 100), request())

        assertEquals(Command.ServerResponse(7, 0, ""), response)
    }

    @Test
    fun aRequestCancelledMidFlightIsStillSettledWithStatusZero() {
        val arrived = CountDownLatch(1)
        val base = serve { arrived.countDown(); Thread.sleep(300); 200 to "too late" }
        val settles = LinkedBlockingQueue<Command.ServerResponse>()

        runBlocking {
            val job = ServerHttp(base).carry(request(), scope) { settles.add(it) }
            assertTrue(arrived.await(5, TimeUnit.SECONDS))
            delay(20)
            job.cancel()
            job.join()
        }

        assertEquals(listOf(Command.ServerResponse(7, 0, "")), settles.toList())
    }
}
