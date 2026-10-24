#include "../Util/BlockReason.h"
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
		using namespace ms::block_reason;

		require(node(1) == 1 && node(20) == 20 && node(41) == 41, "most reasons are their own node");
		require(node(21) == 12 && node(40) == 12, "21 to 40 share 12's");

		require(until(134374144200000000, 2026) == "You can login after\\nOct/25/2026  3:07 pm", "an afternoon");
		require(until(134120466000000000, 2026) == "You can login after\\nJan/ 5/2026  0:30 am", "midnight stays 0");
		require(until(134175315000000000, 2026) == "You can login after\\nMar/ 9/2026 12:05 am", "noon is am");
		require(until(135694656000000000, 2029) == "You can login after\\nJan/ 1/2031  0:00 am", "two years on");
		require(until(135694656000000000, 2028).rfind("The ID has been permanently blocked.", 0) == 0,
				"three years on is forever");

		require(text(0, 134374144200000000, 2026, "msg").empty(), "reason 0 is the plain notice");
		require(text(5, 134374144200000000, 2026, "").empty(), "so is a reason without text");
		require(text(5, 134374144200000000, 2026, "Cursing.") == "Cursing.\\nYou can login after\\nOct/25/2026  3:07 pm",
				"the reason's text, then the date");
		require(text(99, 134374144200000000, 2026, "").rfind("You have been blocked for typing in an invalid password", 0) == 0,
				"99 has its own text");
	}
	catch (const std::exception& e)
	{
		std::cerr << e.what() << std::endl;
		return 1;
	}

	std::cout << "block_reason tests passed" << std::endl;
	return 0;
}
