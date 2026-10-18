#include "../Util/Megaphones.h"
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
		using namespace ms::Megaphones;

		require(heard_on(100000000), "megaphones are heard in Henesys");
		require(!heard_on(109010000), "but not on an event map");

		require(with_item("Hero : selling", "Ilbi") == "Hero : [Ilbi]selling", "the item goes before the text");
		require(with_item("selling", "Ilbi") == "[Ilbi]selling", "even without a name");

		require(under_name("Hero : one", "Hero : two") == "Hero : two", "a later line keeps the name");
		require(under_name("Hero : one", "Other : two : three") == "Hero : two : three",
				"a later line takes the first line's name");
		require(under_name("Hero : one", "two") == "two", "a line without a name stays as it is");
	}
	catch (const std::exception& e)
	{
		std::cerr << "FAIL: " << e.what() << std::endl;
		return 1;
	}

	return 0;
}
