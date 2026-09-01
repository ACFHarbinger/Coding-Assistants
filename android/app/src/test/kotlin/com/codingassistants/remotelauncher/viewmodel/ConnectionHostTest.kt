package com.codingassistants.remotelauncher.viewmodel

import com.codingassistants.remotelauncher.ui.isValidServerHost
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ConnectionHostTest {
    @Test
    fun testParseHostPortDefault() {
        val (host, port) = parseHostPort("192.168.1.100")
        assertEquals("192.168.1.100", host)
        assertEquals(5555, port)
    }

    @Test
    fun testParseHostPortCustomPort() {
        val (host, port) = parseHostPort("192.168.1.100:8080")
        assertEquals("192.168.1.100", host)
        assertEquals(8080, port)
    }

    @Test
    fun testParseHostPortHostname() {
        val (host, port) = parseHostPort("desktop-pc:5556")
        assertEquals("desktop-pc", host)
        assertEquals(5556, port)
    }

    @Test
    fun testParseHostPortInvalidPortFallback() {
        val (host, port) = parseHostPort("10.0.0.5:99999")
        assertEquals("10.0.0.5:99999", host)
        assertEquals(5555, port)
    }

    @Test
    fun testIsValidServerHost() {
        assertTrue(isValidServerHost("192.168.1.1"))
        assertTrue(isValidServerHost("192.168.1.1:5555"))
        assertTrue(isValidServerHost("localhost"))
        assertTrue(isValidServerHost("my-desktop:8080"))
        assertFalse(isValidServerHost(""))
        assertFalse(isValidServerHost("192.168.1.300"))
        assertFalse(isValidServerHost("192.168.1.1:invalid"))
        assertFalse(isValidServerHost("192.168.1.1:70000"))
    }
}
