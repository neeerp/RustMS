#include "../Character/MorphRules.h"

#include <iostream>
#include <stdexcept>

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
}

int main()
{
	using namespace ms::morph;

	try
	{
		require(action("stand1") == "stand" && action("alert") == "stand", "standing and alert stand");
		require(action("walk2") == "walk" && action("proneStab") == "prone", "walks walk and a prone stab lies prone");
		require(action("rope") == "rope" && action("fly") == "fly" && action("sit") == "sit", "climbing, flying and sitting keep their names");
		require(action("swingO1") == "stand", "an attack stands");

		require(speed(80) == 80 && speed(150) == 140 && speed(50) == 80, "speed is 80 to 140");
		require(jump(100) == 100 && jump(130) == 123 && jump(70) == 80, "jump is 80 to 123");

		require(!may_cancel(-2210009, true, true), "a noCancelMouse morph item can't be cancelled while morphed");
		require(may_cancel(-2210000, true, false), "another morph item can");
		require(may_cancel(-2210009, false, true), "nor is it refused when not morphed");
		require(may_cancel(-2022003, true, true), "nor is another item's buff");

		std::cout << "Morph rules passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
