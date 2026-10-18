#include "../Util/CharacterName.h"
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
		using ms::character_name_valid;

		require(character_name_valid("Bob1") && character_name_valid("TwelveLetter"), "4 to 12 letters or digits");
		require(!character_name_valid("Bob") && !character_name_valid("ThirteenChars"), "no fewer, no more");
		require(!character_name_valid("Bob hi") && !character_name_valid("Bob_1") && !character_name_valid("B\xc3\xa9" "bob"),
				"nothing else");
		require(character_name_valid("IllX") && !character_name_valid("IllIX") && !character_name_valid("lllll"),
				"fewer than four of I and l");
		require(character_name_valid("iiiiLLLL"), "other cases don't count");

		std::cout << "Character name passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
