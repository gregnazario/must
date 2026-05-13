package com.example;

import org.junit.jupiter.api.Test;
import static org.junit.jupiter.api.Assertions.*;

class AppTest {
    @Test
    void greetReturnsCorrectMessage() {
        assertEquals("Hello, world!", App.greet("world"));
    }

    @Test
    void greetReturnsCorrectMessageForAnyName() {
        assertEquals("Hello, Mustfile!", App.greet("Mustfile"));
    }
}
