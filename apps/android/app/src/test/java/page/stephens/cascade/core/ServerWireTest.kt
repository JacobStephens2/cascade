package page.stephens.cascade.core

import org.junit.Assert.assertEquals
import org.junit.Test

/** The request wire, as `cascade-core` writes and reads it. */
class ServerWireTest {
    @Test
    fun aServerRequestDecodesFromTheCoresJson() {
        val effect = cascadeJson.decodeFromString<Effect>(
            """{"type":"serverRequest","id":3,"method":"PUT","path":"/listening","bearerToken":"tok","body":"{\"deviceId\":\"d\"}"}""",
        )
        assertEquals(Effect.ServerRequest(3, "PUT", "/listening", "tok", """{"deviceId":"d"}"""), effect)
    }

    @Test
    fun aServerRequestWithoutBearerOrBodyDecodes() {
        val effect = cascadeJson.decodeFromString<Effect>(
            """{"type":"serverRequest","id":4,"method":"DELETE","path":"/account"}""",
        )
        assertEquals(Effect.ServerRequest(4, "DELETE", "/account", null, null), effect)
    }

    @Test
    fun aServerResponseEncodesForTheCore() {
        assertEquals(
            """{"type":"serverResponse","id":3,"status":401,"body":""}""",
            cascadeJson.encodeToString(Command.serializer(), Command.ServerResponse(3, 401, "")),
        )
    }

    @Test
    fun deletesCarryTheNewDeviceId() {
        assertEquals(
            """{"type":"deleteListeningData","newDeviceId":"n"}""",
            cascadeJson.encodeToString(Command.serializer(), Command.DeleteListeningData("n")),
        )
        assertEquals(
            """{"type":"deleteAccount","newDeviceId":"n"}""",
            cascadeJson.encodeToString(Command.serializer(), Command.DeleteAccount("n")),
        )
    }

    @Test
    fun aSignInLinkRequestNamesItsPlatformOnlyWhenGiven() {
        assertEquals(
            """{"type":"requestSignInLink","email":"a@b.c"}""",
            cascadeJson.encodeToString(Command.serializer(), Command.RequestSignInLink("a@b.c")),
        )
        assertEquals(
            """{"type":"requestSignInLink","email":"a@b.c","platform":"android"}""",
            cascadeJson.encodeToString(Command.serializer(), Command.RequestSignInLink("a@b.c", "android")),
        )
    }
}
