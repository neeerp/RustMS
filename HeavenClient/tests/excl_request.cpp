#include "../Character/ExclRequest.h"
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
		using namespace ms;

		ExclRequest excl;
		require(excl.can_send(0, ExclRequest::WAIT), "the first request goes at once");

		excl.sent(1000);
		require(!excl.can_send(5000, ExclRequest::WAIT), "a request waits on its answer");

		excl.answered(5000);
		require(!excl.can_send(5499, ExclRequest::WAIT), "the next request waits after the answer");
		require(excl.can_send(5500, ExclRequest::WAIT), "the next request goes once the wait is over");
		require(excl.can_send(5200, ExclRequest::USE_WAIT), "an item use waits less");
		require(!excl.can_send(5299, ExclRequest::SKILL_WAIT), "a skill waits");
		require(excl.can_send(5300, ExclRequest::SKILL_WAIT), "a skill waits less than a move");
		require(!excl.can_send(5029, ExclRequest::PICKUP_WAIT), "a pickup waits too");
		require(excl.can_send(5030, ExclRequest::PICKUP_WAIT), "a pickup waits least");

		excl.sent(6000);
		excl.clear();
		require(!excl.can_send(6100, ExclRequest::USE_WAIT), "a new field keeps the request's time");
		require(excl.can_send(6200, ExclRequest::USE_WAIT), "a new field forgets the waiting request");

		ExclRequest potions;
		potions.sent(0);
		potions.answered(0);
		require(potions.can_send_potion(0), "an auto potion keeps its own time");
		potions.sent_potion(0, false);
		require(!potions.can_send_potion(1000), "an auto potion waits on its answer too");
		potions.answered(1000);
		require(potions.can_send_potion(1000), "an answer doesn't hold the next potion");
		require(!potions.can_send(1499, ExclRequest::WAIT), "an answer to a potion holds other requests");
		potions.sent_potion(1000, true);
		potions.answered(1000);
		require(potions.can_send_potion(1000), "an MP potion leaves the HP time alone");
		potions.sent_potion(1000, false);
		potions.answered(1000);
		require(!potions.can_send_potion(1199), "an HP potion waits 200 ms");
		require(potions.can_send_potion(1200), "and then may go");
		potions.sent(1300);
		require(!potions.can_send_potion(5000), "any waiting request holds the potion");

		std::cout << "Exclusive request passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
