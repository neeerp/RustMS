#include "../Character/Look/HatSlots.h"
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
		using namespace ms::hat_slots;

		require(covers("CpH1H5", "H1") && covers("CpH1H5", "H5"), "a half cover hides H1 and H5");
		require(!covers("CpH1H5", "H2"), "but not the hair, H2");
		require(!covers("CpH5", "H1") && !covers("CpH5", "H2"), "a headband hides neither front layer");
		require(!covers("Cp", "H1") && !covers("Cp", "H2") && !covers("Cp", "Hs"), "a plain Cp hides no hair");
		require(covers("CpHdH1H2H3H4H5H6HsHfHbAfAyAsAe", "H2"), "a full helmet hides the hair");
		require(covers("CpHdH1H2H3H4H5H6HsHfHbAfAyAsAe", "Hs") && covers("CpHdH1H2H3H4H5H6HsHfHbAfAyAsAe", "Hb"),
				"and its shade and the hair below the body");
		require(!covers("CpH1H4", "H6"), "a slot is matched whole");
		require(!covers("CH1", "H1") && !covers("pH1", "1H"), "slots start at even places");
		require(!covers("", "H1") && !covers("C", "Cp"), "nothing covers nothing");
	}
	catch (const std::exception& e)
	{
		std::cerr << e.what() << std::endl;
		return 1;
	}

	std::cout << "hat_slots tests passed" << std::endl;
	return 0;
}
