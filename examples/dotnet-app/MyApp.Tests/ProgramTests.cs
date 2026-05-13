using Xunit;

namespace MyApp.Tests;

public class ProgramTests
{
    [Fact]
    public void GreetReturnsCorrectMessage()
    {
        Assert.Equal("Hello, world!", Program.Greet("world"));
    }

    [Fact]
    public void GreetReturnsCorrectMessageForAnyName()
    {
        Assert.Equal("Hello, Mustfile!", Program.Greet("Mustfile"));
    }
}
