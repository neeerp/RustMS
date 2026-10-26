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
		require(action("stand1", false) == "stand" && action("alert", false) == "stand", "standing and alert stand");
		require(action("walk2", false) == "walk" && action("proneStab", false) == "prone", "walks walk and a prone stab lies prone");
		require(action("rope", false) == "rope" && action("fly", false) == "fly", "climbing and flying keep their names");
		require(action("sit", false) == "stand", "an item's morph stands on a chair");
		require(action("swingO1", false) == "stand" && action("stabO1", false) == "stand", "an attack stands");

		require(action("alert", true) == "alert" && action("sit", true) == "sit", "a superman shows its alert and sits");
		require(action("stand2", true) == "stand" && action("walk1", true) == "walk", "and stands and walks");
		require(action("stabO1", true) == "stabO1" && action("proneStab", true) == "proneStab", "and stabs");
		require(action("swingO1", true) == "stand", "but stands for an attack it lacks");

		require(superman_action("somersault") == "somersault" && superman_action("snatch") == "snatch", "a skill keeps its name");
		require(superman_action("alert5") == "alert", "alert5 is the alert");
		require(superman_action("swingO1").empty() && superman_action("alert").empty(), "others take the stance's");

		require(superman_refuses(5001003) && superman_refuses(OAK_BARREL) && superman_refuses(5210000), "a superman can't shoot twice or hide");
		require(superman_refuses(1004) && superman_refuses(10001014) && superman_refuses(20001023) && superman_refuses(20011031), "nor ride");
		require(!superman_refuses(1014) && !superman_refuses(20011019) && !superman_refuses(5121003), "but uses the rest");
		require(superman_refuses(1017) && !superman_refuses(10001017), "a beginner's 1017 but not a Noblesse's");

		require(untransformed(5111006, false, 0) && !untransformed(5111006, true, 5111005), "Shockwave wants a transformation");
		require(untransformed(5121004, true, 5111005) && !untransformed(5121005, true, SUPER_TRANSFORMATION), "Demolition and Snatch Super Transformation");
		require(!untransformed(5001001, false, 0), "another skill wants none");

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

		require(is_cash_morph(5300000) && is_cash_morph(5300002), "530xxxx are cash morphs");
		require(!is_cash_morph(2210000), "a morph potion isn't");
		require(!cash_morph_refused(0, 0), "a cash morph is used standing");
		require(cash_morph_refused(3010000, 0), "not on a chair");
		require(cash_morph_refused(0, 1902000), "nor on a tamed monster");
		require(!cash_morph_refused(0, 1932000), "but on another ride");
		require(!potion_refused(2210000, 0, false, 0, 0, false), "a morph potion is used standing");
		require(potion_refused(2210000, 1, false, 0, 0, false), "not while morphed");
		require(potion_refused(2210000, 0, true, 0, 0, false), "nor by a ghost");
		require(potion_refused(2360000, 0, true, 0, 0, false), "and a ghost item neither");
		require(potion_refused(2360000, 0, false, 3010000, 0, false), "nor a ghost item on a chair");
		require(potion_refused(2210000, 0, false, 0, 1932000, false), "nor aboard the Battleship");
		require(potion_refused(2210000, 0, false, 0, 0, true), "nor in a mini room");
		require(!potion_refused(2210000, 0, false, 0, 1902000, false), "a tamed monster is refused elsewhere");
		require(!potion_refused(2000000, 1, true, 3010000, 1932000, true), "other potions are not refused here");
		require(targets_another(2212000) && !targets_another(2211000) && !targets_another(2210000),
				"the Party Potion is used on someone else");
		require(!targets_another(2022000) && !targets_another(2360000), "nothing else is");

		require(ghost_action("stand1") == "ghoststand" && ghost_action("alert") == "ghoststand",
			"a ghost stands, alert or not");
		require(ghost_action("walk2") == "ghostwalk", "and walks");
		require(ghost_action("prone") == "ghostproneStab", "lies as its proneStab");
		require(ghost_action("ladder") == "ghostladder" && ghost_action("rope") == "ghostrope", "climbs");
		require(ghost_action("jump") == "ghostjump" && ghost_action("fly") == "ghostfly", "jumps and swims");
		require(transform_sounds(1002) && transform_sounds(0) && transform_sounds(2210016), "a morph puffs with a sound");
		require(!transform_sounds(1000) && !transform_sounds(1001), "but not Transformation's");

		require(ghost_action("sit") == "ghostsit", "and sits");
		require(ghost_action("swingO1") == "ghoststand", "anything else stands");
		require(ghost_variant(1) == "1" && ghost_variant(2) == "2", "a ghost's value is its variant");
		require(std::find(GHOST_ACTIONS.begin(), GHOST_ACTIONS.end(), ghost_action("sit")) != GHOST_ACTIONS.end(),
			"each action is loaded");

		std::cout << "Morph rules passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
