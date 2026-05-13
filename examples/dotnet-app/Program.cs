namespace MyApp;

public class Program
{
    public static string Greet(string name) => $"Hello, {name}!";

    public static void Main(string[] args)
    {
        Console.WriteLine(Greet("world"));
    }
}
