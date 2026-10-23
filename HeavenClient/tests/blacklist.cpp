#include "../Character/Blacklist.h"
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
		using namespace ms::Blacklist;
		using Names = std::vector<std::string>;

		require(parse("").empty(), "nothing saved, nothing listed");
		require(parse("Bob1,alice,Zed9") == Names({ "Bob1", "alice", "Zed9" }), "the order added");
		require(parse("Bob1,,bad name,abc,Bob1,Carl") == Names({ "Bob1", "Carl" }), "bad names and repeats dropped");
		require(parse("Bob1,BOB1") == Names({ "Bob1", "BOB1" }), "a repeat matches case");

		std::string many;

		for (int i = 0; i < 30; i++)
			many += "Name" + std::to_string(100 + i) + ",";

		require(parse(many).size() == MAX, "26 at most");
		require(format({ "Bob1", "alice" }) == "Bob1,alice" && format({}).empty(), "split by commas");
		require(parse(format({ "Bob1", "alice" })) == Names({ "Bob1", "alice" }), "read back as written");

		require(sorted({ "bobby", "Zed9", "Alice", "alan" }) == Names({ "Alice", "Zed9", "alan", "bobby" }),
				"capitals first, as strcmp");

		Names listed = { "Bob1", "alice" };
		require(check(listed, "Carl", "Self", false) == Refusal::NONE, "a new name");
		require(check(listed, "Ca", "Self", false) == Refusal::INVALID, "too short to be a name");
		require(check(listed, "Car l", "Self", false) == Refusal::INVALID, "no spaces");
		require(check(listed, "Carl", "Self", true) == Refusal::INVALID, "a forbidden word");
		require(check(listed, "Self", "Self", false) == Refusal::SELF, "not yourself");
		require(check(listed, "self", "Self", false) == Refusal::NONE, "yourself is letter for letter");
		require(check(listed, "BOB1", "Self", false) == Refusal::LISTED, "listed in any case");
		require(check(listed, "Bob1", "Bob1", false) == Refusal::SELF, "yourself is checked first");
		require(check(parse(many), "Carl", "Self", false) == Refusal::FULL, "no 27th");

		require(ms::Blacklist::listed(listed, "Bob1") && ms::Blacklist::listed(listed, "alice"), "a sender listed");
		require(!ms::Blacklist::listed(listed, "BOB1") && !ms::Blacklist::listed(listed, "Alice"),
				"a sender is looked up letter for letter");
		require(!ms::Blacklist::listed(Names(), "Bob1"), "nothing listed");

		remove(listed, "Bob1");
		require(listed == Names({ "alice" }), "deleted");
		remove(listed, "Nobody");
		require(listed == Names({ "alice" }), "nothing else");

		std::cout << "Blacklist passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
