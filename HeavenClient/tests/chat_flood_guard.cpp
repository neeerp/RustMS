#include "../IO/Components/ChatFloodGuard.h"

#include <iostream>
#include <stdexcept>

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
}

int main()
{
	using Result = ms::ChatFloodGuard::Result;

	try
	{
		{
			ms::ChatFloodGuard guard;

			require(guard.check("hi", 0) == Result::SEND, "a first line is sent");
			require(guard.check("hi", 1000) == Result::SEND, "a second identical line is sent");
			require(guard.check("hi", 2000) == Result::SEND, "a third identical line is sent");
			require(guard.check("hi", 3000) == Result::REPEATED, "a fourth identical line is refused");
			require(guard.check("other", 5799) == Result::MUTED, "lines within 2.8 s of a warning are dropped");
			require(guard.check("hi", 5800) == Result::REPEATED, "the run of identical lines remains");
			require(guard.check("other", 8601) == Result::SEND, "a different line ends the run");
		}

		{
			ms::ChatFloodGuard guard;

			require(guard.check("a", 0) == Result::SEND, "burst line 1");
			require(guard.check("b", 100) == Result::SEND, "burst line 2");
			require(guard.check("c", 200) == Result::SEND, "burst line 3");
			require(guard.check("d", 300) == Result::TOO_FAST, "a fourth line within 2 s is refused");
			require(guard.check("e", 3099) == Result::MUTED, "the mute lasts 2.8 s");
			require(guard.check("e", 3100) == Result::SEND, "lines are sent again after the mute");
		}

		{
			ms::ChatFloodGuard guard;

			require(guard.check("a", 0) == Result::SEND, "paced line 1");
			require(guard.check("b", 700) == Result::SEND, "paced line 2");
			require(guard.check("c", 1400) == Result::SEND, "paced line 3");
			require(guard.check("d", 2100) == Result::SEND, "four lines over 2.1 s are fine");
		}

		{
			ms::ChatFloodGuard guard;

			require(guard.check("hi", 0) == Result::SEND, "spread line 1");
			require(guard.check("hi", 10000) == Result::SEND, "spread line 2");
			require(guard.check("hi", 20000) == Result::SEND, "spread line 3");
			require(guard.check("hi", 30001) == Result::SEND, "the run starts over 30 s after it began");
		}

		std::cout << "Chat flood guard passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
