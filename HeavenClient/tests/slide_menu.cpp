#include "../IO/UITypes/SlideMenu.h"
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
		using namespace ms::slide_menu;

		Menu menu = parse("#2# Monster Carnival 1#0# Ariant Coliseum#1# Mu Lung Dojo");
		require(menu.entries.size() == 3, "three choices");
		require(menu.entries[0].id == 0 && menu.entries[1].id == 1 && menu.entries[2].id == 2, "sorted by id");
		require(menu.entries[0].label == " Ariant Coliseum", "labels keep their spaces");
		require(menu.start == 1 && !menu.empty, "the second starts in the middle");

		menu = parse("#3#Orbis");
		require(menu.entries.size() == 1 && menu.entries[0].id == 3 && menu.start == 0 && !menu.empty,
			"one choice starts in the middle");

		menu = parse("#-1# There is no place for you to transport to from here.");
		require(menu.empty && menu.entries.size() == 1, "-1 alone is nowhere to go");
		require(menu.entries[0].label == " There is no place for you to transport to from here.",
			"-1 alone keeps its label");

		menu = parse("#-1#Nowhere#4#Ludibrium");
		require(menu.empty && menu.entries.size() == 1 && menu.entries[0].label.empty(),
			"-1 first among others leaves an unlabelled -1");

		menu = parse("#4#Ludibrium#-1#Nowhere");
		require(menu.empty && menu.entries.size() == 1 && menu.entries[0].label.empty(),
			"-1 sorted first does too");

		menu = parse("#4#Ludibrium#");
		require(menu.entries.size() == 1 && menu.entries[0].id == 4, "a closing # ends the list");

		require(parse("Ludibrium").entries.empty(), "text not starting with # has no choices");
		require(parse("").empty, "nothing is empty");

		std::cout << "Slide menu passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
