#include "../Gameplay/QuestAlertRules.h"
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
		using namespace ms::quest_alert;

		// Found on entering a field: the bulb lights without a sound.
		State state;
		require(!offer(state, 2230, true, false, false, 0), "not opened");
		require(shown(state) && !state.new_start, "silent");
		require(!update(state, false, false) && state.bulb == Bulb::DEFAULT, "straight to Default");

		// A level-up finds another: it sounds once, and Appear plays before Default.
		offer(state, 2135, true, false, true, 0);
		require(update(state, false, false) && state.bulb == Bulb::APPEAR, "alert");
		require(!update(state, false, false) && state.bulb == Bulb::APPEAR, "once");
		require(!update(state, false, true) && state.bulb == Bulb::DEFAULT, "then Default");
		offer(state, 2135, true, false, true, 0);
		require(!state.new_start, "a quest offered again is silent");

		// A new alert during Appear sounds without starting it over.
		offer(state, 2300, true, false, true, 0);
		update(state, false, false);
		offer(state, 2301, true, false, true, 0);
		require(update(state, false, false) && state.bulb == Bulb::APPEAR, "sounds during Appear");

		// A failed check takes a quest out; with none left the bulb goes.
		State one;
		offer(one, 2230, true, false, true, 0);
		offer(one, 2230, false, false, true, 0);
		require(!shown(one) && !update(one, false, false) && one.bulb == Bulb::NONE, "gone");
		require(!one.new_start, "the alert went with it");

		// A locked UI holds the alert back, with the bulb up meanwhile.
		State locked;
		offer(locked, 2230, true, false, true, 0);
		require(!update(locked, true, false) && locked.bulb == Bulb::DEFAULT && locked.new_start, "held");
		require(update(locked, false, false) && locked.bulb == Bulb::APPEAR, "sounds after");

		// An autoAccept quest opens itself once, without the bulb, and again 10 s on if it didn't start.
		State accept;
		require(offer(accept, 19001, true, true, true, 500), "opens");
		require(!offer(accept, 19001, true, true, true, 600), "once");
		require(!shown(accept), "no bulb");
		require(!reoffer_due(accept, 10500) && reoffer_due(accept, 10501), "after 10 s");
		accept.accepted.clear();
		require(offer(accept, 19001, true, true, true, 10501), "opens again");
		offer(accept, 19001, false, true, true, 10600);
		require(!reoffer_due(accept, 30000) && offer(accept, 19001, true, true, true, 30000), "fails, then opens");

		// autoComplete quests sound when they newly pass and keep their order.
		State done;
		check_complete(done, 2313, true);
		require(done.new_complete && update(done, false, false), "complete alert");
		check_complete(done, 2313, true);
		require(!done.new_complete, "once");
		check_complete(done, 2100, true);
		check_complete(done, 2050, true);
		require(done.completable == std::vector<int16_t>({ 2313, 2100, 2050 }), "in order found");
		check_complete(done, 2100, false);
		require(done.completable == std::vector<int16_t>({ 2313, 2050 }), "a failed check takes it out");

		// Records: started leaves the startable quests, and joins the pre-complete ones if autoPreComplete;
		// forfeited or completed leaves all.
		State records;
		offer(records, 2230, true, false, false, 0);
		offer(records, 2135, true, false, false, 0);
		check_complete(records, 2313, true);
		on_record(records, 2230, 1, true);
		on_record(records, 2135, 1, false);
		require(records.startable.empty() && records.pre_complete == std::set<int16_t>({ 2230 }), "started");
		on_record(records, 2230, 2, true);
		on_record(records, 2313, 0, false);
		require(!shown(records), "completed and forfeited");

		// A new field finds them again from scratch.
		State field;
		offer(field, 2230, true, false, true, 0);
		field.pre_complete.insert(2135);
		update(field, false, false);
		rescan(field);
		require(field.startable.empty() && field.pre_complete.empty() && field.bulb == Bulb::NONE, "rescan");

		// The menu: completable quests under list3, startable under list1, pre-complete under list0.
		State listed;
		offer(listed, 2301, true, false, false, 0);
		offer(listed, 2300, true, false, false, 0);
		listed.pre_complete.insert(2230);
		check_complete(listed, 2313, true);
		auto entries = menu(listed);
		require(entries.size() == 4, "four");
		require(entries[0].quest == 2313 && entries[0].header == 3, "completable first");
		require(entries[1].quest == 2300 && entries[2].quest == 2301 && entries[1].header == 1, "startable");
		require(entries[3].quest == 2230 && entries[3].header == 0, "in progress last");
	}
	catch (const std::exception& e)
	{
		std::cerr << "FAIL: " << e.what() << std::endl;
		return 1;
	}

	return 0;
}
