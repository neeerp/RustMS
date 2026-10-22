#include "../IO/Components/FloatingLines.h"
#include <iostream>
#include <stdexcept>

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
}

int main()
{
	try
	{
		ms::FloatingLines lines;

		require(lines.opacity(0) == 0.0f, "an unused slot shows nothing");

		for (size_t i = 0; i < ms::FloatingLines::SLOTS; i++)
			require(lines.add("line " + std::to_string(i)) == i, "lines fill the slots in order");

		require(!lines.add("sixth"), "with every slot busy a line is dropped");
		require(lines.text(0) == "line 0", "the dropped line overwrote nothing");

		lines.update(1000);
		require(lines.opacity(0) == 1.0f, "a line is solid at first");
		require(!lines.add("still busy"), "nothing frees within 3 s");

		lines.update(2000);
		require(lines.opacity(0) == 1.0f, "still solid at 3 s");
		require(!lines.add("at 3 s"), "a slot frees only after 3 s");
		lines.update(1500);
		require(lines.opacity(0) == 0.5f, "half faded 1.5 s later");

		require(lines.add("next") == size_t(0), "a fading slot takes a new line");
		require(lines.text(0) == "next" && lines.opacity(0) == 1.0f, "the new line starts solid");
		require(lines.opacity(1) == 0.5f, "the others carry on fading");

		lines.update(1500);
		require(lines.opacity(1) == 0.0f, "gone after 6 s");
		lines.update(100000);
		require(lines.opacity(1) == 0.0f, "and stays gone");
		require(lines.opacity(0) == 0.0f, "the newer line has gone too");

		std::cout << "floating lines tests passed\n";
		return 0;
	}
	catch (const std::exception& e)
	{
		std::cerr << e.what() << '\n';
		return 1;
	}
}
