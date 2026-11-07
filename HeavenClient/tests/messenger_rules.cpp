#include "../IO/MessengerRules.h"

#include <iostream>
#include <stdexcept>

using namespace ms::messenger_rules;

namespace
{
	void require(bool condition, const char *message)
	{
		if (!condition)
			throw std::runtime_error(message);
	}

	void typed_lines()
	{
		require(read("hello").command == Command::CHAT, "a plain line is said");
		require(read("hi /q").command == Command::CHAT, "a slash later on is said");
		require(read("/q").command == Command::QUIT, "/q leaves");
		require(read("/Q").command == Command::QUIT, "/Q leaves");
		require(read("/q ").command == Command::NONE, "/q must be the whole line");
		require(read("/quit").command == Command::NONE, "/quit is nothing");

		Typed invite = read("/INVITE Bob");
		require(invite.command == Command::INVITE, "/INVITE invites");
		require(invite.name == "Bob", "the name keeps its case");
		require(read("/invite  Bob").name == " Bob", "the name isn't trimmed");
		require(read("/invite ").command == Command::NONE, "/invite needs a name");
		require(read("/invite").command == Command::NONE, "/invite alone is nothing");
		require(read("/inviteBob").command == Command::NONE, "the space is needed");
		require(read("/wat").command == Command::NONE, "an unknown slash line goes nowhere");
	}

	void heard_lines()
	{
		Heard chat = hear("Bob : hello : there");
		require(chat.chat && !chat.typing, "a chat line ends the typing");
		require(chat.name == "Bob", "the name ends at the first separator");

		Heard started = hear("Bob1");
		require(started.name == "Bob" && started.typing && !started.chat, "1 is typing");

		Heard stopped = hear("Bob0");
		require(stopped.name == "Bob" && !stopped.typing && !stopped.chat, "0 stopped");

		require(hear("").name.empty(), "an empty line names nobody");
	}

	void gate_waits()
	{
		using Notice = TypingGate::Notice;

		TypingGate gate;

		require(gate.step(false, 8) == Notice::NONE, "nothing to tell");
		require(gate.step(true, 8) == Notice::STARTED, "the first letter tells the room");
		require(gate.step(false, 2992) == Notice::NONE, "3 seconds pass between notices");
		require(gate.step(false, 8) == Notice::NONE, "the last of the wait");
		require(gate.step(false, 8) == Notice::STOPPED, "an emptied input tells the room");

		TypingGate said;

		said.step(true, 3000);
		said.sent();
		said.step(false, 3000);
		require(said.step(false, 8) == Notice::NONE, "a chat line needs no stop notice");
		require(said.step(true, 8) == Notice::STARTED, "the next letter tells the room again");

		TypingGate idle;

		idle.arrival();
		require(idle.step(false, 8) == Notice::STOPPED, "someone new hears we aren't typing");

		TypingGate typing;

		typing.step(true, 8);
		typing.arrival();
		require(typing.step(true, 8) == Notice::STARTED, "someone new hears at once that we are");
	}

	void history_walks()
	{
		History history;
		std::string line = "kept";

		require(!history.older(line) && line == "kept", "nothing yet");

		history.push("one");
		history.push("two");

		require(history.older(line) && line == "two", "up finds the last line");
		require(history.older(line) && line == "one", "up again the one before");
		require(!history.older(line), "the oldest stays");
		require(history.newer(line) && line == "two", "down comes back");
		require(!history.newer(line), "the newest stays");
	}

	void typing_rows()
	{
		require(typing_row({}).empty(), "nobody typing, no row");
		require(typing_row({ "a" }) == "a is typing.", "one name");
		require(typing_row({ "a", "b" }) == "a,b is typing.", "names join with a comma");
	}

	void names()
	{
		require(same_name("MageB", "mageb"), "case doesn't matter");
		require(!same_name("mage", "mageb"), "length does");
	}
}

int main()
{
	try
	{
		typed_lines();
		heard_lines();
		gate_waits();
		history_walks();
		typing_rows();
		names();
	}
	catch (const std::exception &e)
	{
		std::cerr << e.what() << '\n';

		return 1;
	}

	std::cout << "Messenger rules passed\n";

	return 0;
}
