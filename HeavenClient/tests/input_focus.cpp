#include "../IO/InputFocus.h"
#include "../IO/KeyAction.h"
#include <iostream>
#include <stdexcept>
#include <vector>

struct Panel
{
	bool active = false;
	std::vector<int32_t> received;
	bool is_active() const { return active; }
	void send_key(int32_t action, bool, bool) { received.push_back(action); }
};

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
}

int main()
{
	try
	{
		constexpr int32_t right = ms::KeyAction::RIGHT, jump = ms::KeyAction::JUMP;
		Panel panel;
		bool walking = false;
		bool running_jump = false;
		bool jumped = false;
		auto gameplay = [&](int32_t action, bool down)
		{
			if (action == right) walking = down;
			if (action == jump && down) { jumped = true; running_jump = walking; }
		};
		send(&panel, jump, true);
		require(jumped, "inactive focus discarded the first jump press");
		auto send = [&](Panel* focused, int32_t action, bool down)
		{
			ms::dispatch_focused_key(focused, action, down, false, [&] { gameplay(action, down); });
		};
		// Reproduce an inactive shop in the focus slot on world entry: the
		// initial direction press must arrive before jump, without key repeat.
		send(&panel, right, true);
		send(nullptr, jump, true);
		require(running_jump, "inactive focus discarded the first walk press before jumping");
		require(panel.received.empty(), "inactive panels must not receive gameplay input");
		// A release after closing a modal must also reach gameplay, otherwise
		// the player keeps walking after the physical direction key is released.
		send(&panel, right, false);
		require(!walking, "inactive focus discarded the direction-key release");
		panel.active = true;
		send(&panel, right, true);
		require(!walking, "active modal leaked movement input into gameplay");
		require(panel.received.size() == 1 && panel.received[0] == right, "active modal did not receive the key");
		panel.active = false;
		send(&panel, right, true);
		require(walking, "closing the modal did not restore movement immediately");
		std::cout << "Keyboard focus transitions passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
