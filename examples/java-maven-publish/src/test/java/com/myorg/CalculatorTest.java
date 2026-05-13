package com.myorg;

import org.junit.jupiter.api.Test;
import static org.junit.jupiter.api.Assertions.*;

class CalculatorTest {
    @Test
    void add() {
        assertEquals(5, Calculator.add(2, 3));
    }

    @Test
    void greet() {
        assertEquals("Hello, world!", Calculator.greet("world"));
    }
}
