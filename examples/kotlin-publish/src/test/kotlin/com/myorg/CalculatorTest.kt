package com.myorg

import kotlin.test.Test
import kotlin.test.assertEquals

class CalculatorTest {
    @Test
    fun add() {
        assertEquals(5, add(2, 3))
    }

    @Test
    fun greet() {
        assertEquals("Hello, world!", greet("world"))
    }
}
