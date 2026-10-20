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

		require(spotted(37, 37) && spotted(37, 137) && !spotted(37, 38), "a mob spots the barrel prop + 1 in 100");
		require(spotted(0, 0) && !spotted(0, 1), "even at prop 0, one in 100");

		Barrel barrel;
		require(!barrel.busy() && barrel.alpha() == 1.0f, "no barrel is idle");

		barrel.drop(1002);
		require(barrel.dropping() && barrel.alpha() == 1.0f, "the body shows while the barrel drops");

		for (int i = 0; i < 27; i++)
			require(barrel.update(30) == 0, "the barrel is still dropping");

		require(barrel.update(30) == 1002 && !barrel.busy(), "it lands after 840 ms");
		require(barrel.update(30) == 0, "and lands once");

		barrel.crack();
		require(barrel.busy() && !barrel.dropping(), "the barrel breaks");
		barrel.update(30);
		require(barrel.alpha() == 0.3f, "the body shows faintly while the barrel breaks");

		for (int i = 0; i < 4; i++)
			barrel.update(30);

		require(barrel.alpha() == 0.7f, "then clearer, from 120 ms left");

		for (int i = 0; i < 3; i++)
			require(barrel.update(30) == 0, "breaking lands nothing");

		require(!barrel.busy() && barrel.alpha() == 1.0f, "and solid after 240 ms");

		barrel.drop(1002);
		barrel.stop();
		require(!barrel.busy() && barrel.update(30) == 0, "a barrel stopped never lands");

		std::cout << "Morph rules passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
