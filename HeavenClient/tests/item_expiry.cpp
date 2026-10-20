#include "../Character/Inventory/ItemExpiry.h"
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
		using namespace ms::item_expiry;

		constexpr int64_t OCT_19_2026_13_05 = 134368887000000000;
		constexpr int64_t SECOND = TICKS_PER_SECOND;

		require(line(OCT_19_2026_13_05, false, false) == "USABLE UNTIL 10-19-2026 AT 13 : 05", "usable");
		require(line(OCT_19_2026_13_05, false, true) == "Seal Released on 10-19-2026 13:05", "sealed");
		require(line(OCT_19_2026_13_05, true, true) == "Available until 10/19/2026 at 13:05", "cash comes first");
		require(line(OCT_19_2026_13_05 + 59 * SECOND, false, false) == "USABLE UNTIL 10-19-2026 AT 13 : 05",
			"seconds dropped");

		require(line(BEFORE, true, false).empty() && line(0, true, false).empty() && line(AFTER, true, false).empty(),
			"no line for none");
		require(line(BEFORE - 1, false, false) == "USABLE UNTIL 12-31-2078 AT 23 : 59", "just before 2079");
		require(line(AFTER + 1, false, false) == "USABLE UNTIL 1-1-1900 AT 00 : 00", "just after 1900");
		require(line(116444735400000000, true, false) == "Available until 12/31/1969 at 23:59", "before 1970");
		require(line(125962830000000000, true, false) == "Available until 2/29/2000 at 07:30", "a leap day");

		PetLife days{};
		PetLife permanent{ true, 0 };
		PetLife limited{ false, 18000 };

		require(pet_line(OCT_19_2026_13_05, 0, days) == "WATER OF LIFE DRIES UP ON 10-19-2026 AT 13 : 00",
			"pet date, minutes dropped");
		require(pet_line(OCT_19_2026_13_05, 0, permanent) == "PERMANENT", "permanent pet");
		require(pet_line(BEFORE, 0, days) == "THE WATER OF LIFE HAS DRIED UP." && pet_dead(BEFORE, 0, days),
			"dated 2079 is dead");
		require(pet_line(BEFORE, 0, permanent) == "THE WATER OF LIFE HAS DRIED UP.", "dead before permanent");
		require(!pet_dead(AFTER, 0, days), "a past date isn't dead");
		require(pet_line(BEFORE, 18000, limited) == "DAYS OF MAGIC : 5hrs. 0min." && !pet_dead(BEFORE, 18000, limited),
			"limited life counts seconds");
		require(pet_line(BEFORE, 3725, limited) == "DAYS OF MAGIC : 1hrs. 2min.", "hours and minutes");
		require(pet_dead(OCT_19_2026_13_05, 0, limited), "limited life spent");
		require(pet_line(0, 18000, limited).empty() && pet_line(0, 0, days).empty(), "no date, no line");

		std::cout << "item expiry passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
