#include "../IO/KeyRepeatRules.h"
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
		using namespace ms::key_repeat;

		Repeater repeat;
		require(repeat.update(1000) == NONE, "nothing repeats before a key goes down");

		repeat.press(341, 1000);
		require(repeat.update(1499) == NONE, "a held key waits out the delay");
		require(repeat.update(1500) == 341, "then goes down again");
		require(repeat.update(1532) == NONE, "the next waits the interval");
		require(repeat.update(1533) == 341, "and comes 33 ms on");
		require(repeat.update(1580) == 341, "a late one times the next from itself");
		require(repeat.update(1612) == NONE, "not from when it was due");
		require(repeat.update(1613) == 341, "33 ms after the late one");

		repeat.press(65, 2000);
		require(repeat.update(2499) == NONE, "a new key starts the delay over");
		require(repeat.update(2500) == 65, "and repeats in place of the first");

		repeat.release(341);
		require(repeat.update(2533) == 65, "letting go of the first changes nothing");

		repeat.release(65);
		require(repeat.update(3000) == NONE, "letting go of the repeating key ends it");

		repeat.press(341, 4000);
		repeat.stop();
		require(repeat.update(5000) == NONE, "losing focus ends it");
	}
	catch (const std::exception& e)
	{
		std::cerr << "FAIL: " << e.what() << std::endl;
		return 1;
	}

	return 0;
}
