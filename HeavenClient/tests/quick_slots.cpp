#include "../IO/Keyboard.h"
#include "../IO/QuickSlots.h"

#include <GLFW/glfw3.h>

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
		using Assign = QuickSlots::Assign;

		QuickSlots::Keys keys = QuickSlots::DEFAULTS;

		require(QuickSlots::assign(keys, 0, 30) == Assign::OK, "A is a quick slot key");
		require(keys[0] == 30, "the cell takes the key");
		require(QuickSlots::assign(keys, 1, 30) == Assign::IN_USE, "a key goes on one cell");
		require(QuickSlots::assign(keys, 0, 30) == Assign::IN_USE, "pressing a cell's own key is refused too");
		require(keys[1] == 82, "a refused key leaves the cell alone");
		require(QuickSlots::assign(keys, 2, 14) == Assign::DISABLED, "Backspace has no label");
		require(QuickSlots::assign(keys, 2, 1) == Assign::DISABLED, "Esc has no label");
		require(QuickSlots::assign(keys, 2, 53) == Assign::DISABLED, "nor has /");
		require(QuickSlots::assign(keys, 2, 0) == Assign::DISABLED, "an unknown key is refused");
		require(QuickSlots::assign(keys, 2, 88) == Assign::OK, "F12 is a quick slot key");

		keys = QuickSlots::DEFAULTS;
		require(QuickSlots::assign(keys, 3, QuickSlots::RIGHT_SHIFT) == Assign::IN_USE,
				"Right Shift is Shift, which the first cell has");
		keys[0] = 30;
		require(QuickSlots::assign(keys, 1, QuickSlots::RIGHT_SHIFT) == Assign::OK, "Right Shift is free once Shift is");
		require(keys[1] == QuickSlots::LEFT_SHIFT, "and is stored as Shift");

		require(Keyboard::quickslot_key(GLFW_KEY_A) == 30, "A is key 30");
		require(Keyboard::quickslot_key(GLFW_KEY_1) == 2, "1 is key 2");
		require(Keyboard::quickslot_key(GLFW_KEY_KP_1) == 2, "keypad 1 is 1");
		require(Keyboard::quickslot_key(GLFW_KEY_KP_9) == 10, "keypad 9 is 9");
		require(Keyboard::quickslot_key(GLFW_KEY_KP_0) == 11, "keypad 0 is 0");
		require(Keyboard::quickslot_key(GLFW_KEY_KP_DECIMAL) == 83, "keypad . is Delete");
		require(Keyboard::quickslot_key(GLFW_KEY_RIGHT_CONTROL) == 29, "Right Ctrl is Ctrl");
		require(Keyboard::quickslot_key(GLFW_KEY_RIGHT_ALT) == 56, "Right Alt is Alt");
		require(Keyboard::quickslot_key(GLFW_KEY_RIGHT_SHIFT) == QuickSlots::RIGHT_SHIFT, "Right Shift is 54");
		require(Keyboard::quickslot_key(GLFW_KEY_ESCAPE) == 1, "Esc is v83's 1, not the table's 84");
		require(Keyboard::quickslot_key(GLFW_KEY_INSERT) == 82, "Insert is 82");
		require(Keyboard::quickslot_key(GLFW_KEY_F11) == 87, "F11 is 87");
		require(Keyboard::quickslot_key(GLFW_KEY_BACKSPACE) == 0, "Backspace is not in the table");

		QuickSlots slots;
		require(slots.keys() == QuickSlots::DEFAULTS, "slots start on the defaults");
		require(!slots.changed(), "and unchanged");

		QuickSlots::Keys layout = { 30, 31, 32, 33, 54, 35, 36, 37 };
		slots.init(layout);
		require(!slots.changed(), "the server's layout is the saved one");
		require(slots.key(4) == QuickSlots::LEFT_SHIFT, "a slot on Right Shift fires Shift's binding");

		QuickSlots::Keys edited = layout;
		edited[0] = 16;
		slots.set(edited);
		require(slots.changed(), "the editor's OK changes the layout");
		slots.revert();
		require(slots.keys() == layout && !slots.changed(), "closing Key Config takes it back");

		slots.set(edited);
		slots.save();
		require(!slots.changed(), "sending it saves it");
		slots.revert();
		require(slots.keys() == edited, "so a later revert keeps it");

		slots.reset();
		require(slots.keys() == QuickSlots::DEFAULTS && slots.changed(), "Default puts the defaults back");

		std::cout << "QuickSlots passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
