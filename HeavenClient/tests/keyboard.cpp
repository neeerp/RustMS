#include "../IO/Keyboard.h"
#include "../IO/KeyConfig.h"

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
		using Key = KeyConfig::Key;

		const Keyboard::Mapping attack(KeyType::Id::ACTION, KeyAction::Id::ATTACK);
		const Keyboard::Mapping jump(KeyType::Id::ACTION, KeyAction::Id::JUMP);
		const Keyboard::Mapping pickup(KeyType::Id::ACTION, KeyAction::Id::PICKUP);
		const Keyboard::Mapping face(KeyType::Id::FACE, KeyAction::Id::FACE1);

		Keyboard keyboard;
		keyboard.assign(Key::LEFT_CONTROL, KeyType::Id::ACTION, KeyAction::Id::ATTACK);
		keyboard.assign(Key::LEFT_ALT, KeyType::Id::ACTION, KeyAction::Id::JUMP);
		keyboard.assign(Key::LEFT_SHIFT, KeyType::Id::ACTION, KeyAction::Id::PICKUP);
		keyboard.assign(Key::F11, KeyType::Id::FACE, KeyAction::Id::FACE1);

		require(keyboard.get_mapping(GLFW_KEY_RIGHT_CONTROL) == attack, "Right Ctrl does what Ctrl does");
		require(keyboard.get_mapping(GLFW_KEY_RIGHT_ALT) == jump, "Right Alt does what Alt does");
		require(keyboard.get_mapping(GLFW_KEY_RIGHT_SHIFT) == pickup, "Right Shift does what Shift does");
		require(keyboard.get_mapping(GLFW_KEY_LEFT_CONTROL) == attack, "Left Ctrl keeps its binding");
		require(keyboard.get_mapping(GLFW_KEY_F11) == face, "F11 is v83's key 87");

		keyboard.assign(69, KeyType::Id::ACTION, KeyAction::Id::JUMP);
		keyboard.assign(70, KeyType::Id::ACTION, KeyAction::Id::JUMP);
		require(keyboard.get_mapping(GLFW_KEY_F11) == face, "key 69 is not F11");
		require(keyboard.get_mapping(GLFW_KEY_F12).type == KeyType::Id::NONE, "key 70 is not F12");

		require(KeyConfig::fold(Key::RIGHT_CONTROL) == Key::LEFT_CONTROL, "Right Ctrl's face is Ctrl");
		require(KeyConfig::fold(Key::RIGHT_ALT) == Key::LEFT_ALT, "Right Alt's face is Alt");
		require(KeyConfig::fold(Key::RIGHT_SHIFT) == Key::LEFT_SHIFT, "Right Shift's face is Shift");
		require(KeyConfig::fold(Key::F11) == Key::F11, "other keys are their own");

		std::cout << "Keyboard passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
