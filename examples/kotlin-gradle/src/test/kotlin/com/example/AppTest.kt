package com.example

import kotlin.test.Test
import kotlin.test.assertEquals

class AppTest {
    @Test
    fun greetReturnsCorrectMessage() {
        assertEquals("Hello, world!", greet("world"))
    }

    @Test
    fun greetReturnsCorrectMessageForAnyName() {
        assertEquals("Hello, Mustfile!", greet("Mustfile"))
    }
}
