using MyLib;
using Xunit;

namespace MyLib.Tests;

public class CalculatorTests
{
    [Fact]
    public void Add_ReturnsSum()
    {
        Assert.Equal(5, Calculator.Add(2, 3));
    }

    [Fact]
    public void Greet_ReturnsGreeting()
    {
        Assert.Equal("Hello, world!", Calculator.Greet("world"));
    }
}
