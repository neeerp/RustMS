#include "../Character/Inventory/RemoteGachapon.h"
#include <iostream>
#include <set>
#include <stdexcept>
#include <utility>

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
}

int main()
{
	try
	{
		using namespace ms::remote_gachapon;

		require(is_ticket(TICKET) && is_ticket(5451999), "5451xxx are tickets");
		require(!is_ticket(5450000) && !is_ticket(5220000), "Miu Miu and the Gachapon Ticket aren't");

		// The background is 246x235 and a town's button 103x18 (UIClassic.nx).
		std::set<std::pair<int16_t, int16_t>> places;
		for (Place town : TOWNS)
		{
			require(town.x >= 0 && town.x + 103 <= 246 && town.y >= 0 && town.y + 18 <= 235, "a town fits");
			places.insert({ town.x, town.y });
		}
		require(places.size() == TOWNS.size(), "no two towns overlap");
		require(TOWNS[0].x == 15 && TOWNS[0].y == 113, "Henesys sends 0 from the top left");
		require(TOWNS[4].x == 128 && TOWNS[4].y == 155, "Sleepywood sends 4 from the third row's right");
		require(TOWNS[9].x == 15 && TOWNS[9].y == 155, "Nautilus Harbor sends 9 from the third row's left");
		require(CLOSE.x + 12 <= 246, "the close button fits");
	}
	catch (const std::exception& e)
	{
		std::cerr << "FAILED: " << e.what() << std::endl;
		return 1;
	}

	std::cout << "remote_gachapon tests passed" << std::endl;
	return 0;
}
