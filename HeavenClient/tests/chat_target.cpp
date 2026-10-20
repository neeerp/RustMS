#include "../IO/Components/ChatTarget.h"

#include <deque>
#include <iostream>
#include <stdexcept>
#include <string>
#include <vector>

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
}

int main()
{
	using ms::chat_target::Target;
	using ms::chat_target::face_text;
	using ms::chat_target::next_tab;

	try
	{
		require(std::string(ms::chat_target::item_text(Target::BUDDY)) == "To a buddy", "item 0 is buddies");
		require(std::string(ms::chat_target::item_text(Target::WHISPER)) == "Whisper", "item 6 is whisper");
		require(std::string(ms::chat_target::item_text(Target::ALL)) == "To All", "item 7 is everyone");

		require(face_text(Target::GROUP, "", "") == "To Group", "no group shows the item");
		require(face_text(Target::GROUP, "Friends", "") == "[Friends]", "a group shows in brackets");
		require(face_text(Target::GROUP, "TwelveLetter", "") == "[TwelveLetter]", "twelve characters stay whole");
		require(face_text(Target::GROUP, "ThirteenChars", "") == "[Thirteen...]", "longer groups are cut to eight");
		require(face_text(Target::WHISPER, "", "") == "Whisper", "no name shows the item");
		require(face_text(Target::WHISPER, "", "Writs") == "Writs", "a name shows itself");
		require(face_text(Target::PARTY, "Friends", "Writs") == "To the party", "other modes show their item");

		Target target = Target::ALL;
		const Target cycle[] = {
			Target::BUDDY, Target::SPOUSE, Target::GROUP, Target::PARTY,
			Target::GUILD, Target::ALLIANCE, Target::WHISPER, Target::ALL
		};

		for (Target expected : cycle)
		{
			target = next_tab(target);
			require(target == expected, "Tab follows v83's order");
		}

		using ms::chat_target::group_choices;
		using Groups = std::vector<std::string>;
		require(group_choices({}) == Groups({ "" }), "a blank entry with no buddies");
		require(group_choices({ "Default Group", "", "Default Group" }) == Groups({ "", "Default Group" }),
				"each group once, and no second blank");
		require(group_choices({ "b", "A", "a", "B" }) == Groups({ "", "A", "B", "a", "b" }), "bytewise order");

		ms::chat_target::WhisperHistory history;
		require(history.latest().empty(), "an empty history has no latest name");

		for (int i = 0; i < 12; i++)
			history.add("n" + std::to_string(i));

		require(history.all().size() == 10, "the history keeps ten names");
		require(history.latest() == "n11", "the latest name is first");
		require(history.all().back() == "n2", "the oldest names drop off");

		history.add("n5");
		require(history.all().size() == 10, "a repeat isn't added twice");
		require(history.latest() == "n5", "a repeat moves to the front");
		require(history.all()[1] == "n11", "the rest keep their order");

		history.add("N5");
		require(history.all().size() == 10 && history.latest() == "N5", "names are compared exactly");

		history.replace({ "b", "a" });
		require(history.all() == std::deque<std::string>({ "b", "a" }), "the prompt's list replaces the history");

		history.replace({});
		require(history.latest().empty(), "every name can be dropped");
	}
	catch (const std::exception& e)
	{
		std::cerr << e.what() << std::endl;
		return 1;
	}

	return 0;
}
